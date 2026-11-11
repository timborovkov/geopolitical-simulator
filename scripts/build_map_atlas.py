"""Compile offline global reference geography. Install map-requirements.txt to rebuild.

--source-dir contains pinned vector checkout and terrain.tif; --check rebuilds into
an isolated directory and compares bytes. --validate needs only standard Python.
"""
import argparse
import hashlib
import json
from pathlib import Path
import tempfile

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'data/map-atlas'
VECTOR_REVISION = 'ca96624a56bd078437bca8184e78163e5039ad19'
RASTER_REVISION = 'f04832c819e7a203d159dffe93cd9245332d6b33'
LAYERS = {'countries': ('cultural', 'admin_0_countries'), 'cities': ('cultural', 'populated_places'),
          'urban': ('cultural', 'urban_areas'), 'roads': ('cultural', 'roads'),
          'rail': ('cultural', 'railroads'), 'rivers': ('physical', 'rivers_lake_centerlines'),
          'lakes': ('physical', 'lakes')}


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, separators=(',', ':'), allow_nan=False)+'\n', encoding='utf-8', newline='\n')


def validate(directory=OUT):
    manifest = json.loads((directory/'manifest.json').read_text())
    if manifest['version'] != 1:
        raise ValueError('Unsupported atlas version')
    size = 0
    for name, checksum in manifest['files'].items():
        if Path(name).is_absolute() or '..' in Path(name).parts:
            raise ValueError('Unsafe asset path')
        path = directory/name
        if digest(path) != checksum:
            raise ValueError(f'Asset checksum mismatch: {name}')
        size += path.stat().st_size
    if size > 250_000_000:
        raise ValueError('Atlas exceeds 250 MB')
    atlas = json.loads((directory/'atlas.json').read_text())
    ids = [c['id'] for c in atlas['countries']]
    if len(ids) != len(set(ids)) or len(ids) < 200:
        raise ValueError('Invalid global country identities')
    for country in atlas['countries']:
        for lod in country['lods']:
            for polygon in lod:
                points = polygon['points']
                if any(not(-180.001 <= x <= 180.001 and -90.001 <= y <= 90.001) for x,y in points):
                    raise ValueError('Invalid coordinate')
                if len(polygon['triangles']) % 3 or any(i < 0 or i >= len(points) for i in polygon['triangles']):
                    raise ValueError('Invalid mesh index')
                for ring in polygon['rings']:
                    if len(ring) < 4 or ring[0] != ring[-1]:
                        raise ValueError('Unclosed ring')
                    if any(abs(a[0]-b[0]) > 180 for a,b in zip(ring,ring[1:])):
                        raise ValueError('Unsplit antimeridian edge')
    for city in atlas['cities']:
        if city['country'] not in ids:
            raise ValueError('Settlement has unknown reference country')
    print(f"Atlas OK: {len(ids)} reference countries, {len(atlas['cities'])} settlements, {size / 1e6:.1f} MB")


def build(source, output):
    import numpy as np
    import shapefile
    import shapely
    import mapbox_earcut
    from PIL import Image
    Image.MAX_IMAGE_PIXELS = 250_000_000
    lock=ROOT/'data/map-atlas-sources.json'
    if lock.exists():
        for name, expected in json.loads(lock.read_text()).items():
            if digest(source/name)!=expected: raise ValueError(f'Unpinned source: {name}')
    output.mkdir(parents=True, exist_ok=True)
    source_hashes = {}
    def reader(layer):
        folder, name = LAYERS[layer]
        stem = source/'ne-vector'/f'10m_{folder}'/f'ne_10m_{name}'
        for ext in ['shp','shx','dbf','prj']:
            p = stem.with_suffix('.'+ext)
            source_hashes[p.relative_to(source).as_posix()] = digest(p)
        return shapefile.Reader(str(stem), encoding='utf-8', encodingErrors='replace')
    def clean(shape):
        return shapely.make_valid(shapely.geometry.shape(shape.__geo_interface__))
    def parts(g):
        if g.geom_type == 'Polygon': return [g]
        return [p for x in getattr(g, 'geoms', []) for p in parts(x)]
    def mesh(g):
        result=[]
        for p in parts(g):
            rings = [[[round(x,5),round(y,5)] for x,y,*_ in r.coords] for r in [p.exterior,*p.interiors]]
            for ring in rings:
                for i in range(len(ring)-2,-1,-1):
                    a,b=ring[i],ring[i+1]
                    if abs(a[0]-b[0])>180 and abs(a[1])==90 and a[1]==b[1]:
                        ring.insert(i+1,[0.0,a[1]])
            points=[q for ring in rings for q in ring[:-1]]
            if len(points)<3: continue
            ends=np.cumsum([len(r)-1 for r in rings],dtype=np.uint32)
            triangles=mapbox_earcut.triangulate_float64(np.array(points,dtype=np.float64),ends).tolist()
            result.append(dict(rings=rings,points=points,triangles=triangles,bounds=list(p.bounds)))
        return result
    rows=list(reader('countries').iterShapeRecords())
    geometries=[clean(r.shape) for r in rows]
    # Coverage simplification operates on shared edges across the entire collection.
    lods=[shapely.coverage_simplify(geometries,t) for t in [0.12,0.035,0.008]]
    countries=[]
    for i,row in enumerate(rows):
        a=row.record.as_dict(); p=geometries[i].representative_point()
        countries.append(dict(id=str(a['NE_ID']),code=a['ADM0_A3'],name=a['NAME_LONG'],
                              anchor=[a.get('LABEL_X',p.x),a.get('LABEL_Y',p.y)],area=geometries[i].area,rank=int(a['LABELRANK']),lods=[mesh(lod[i]) for lod in lods]))
    by_code={c['code']:c['id'] for c in countries}
    cities=[]
    for row in reader('cities').iterShapeRecords():
        a=row.record.as_dict(); code=a.get('ADM0_A3'); country=by_code.get(code)
        if country is None: continue
        cities.append(dict(id=str(a['NE_ID']),name=a['NAME'],country=country,
                           position=list(row.shape.points[0]),rank=int(a['SCALERANK']),
                           capital=bool(a.get('ADM0CAP',0))))
    cities.sort(key=lambda c:(not c['capital'],c['rank'],c['id']))
    lines=[]
    for layer in ['rivers','roads','rail','urban','lakes']:
        for row in reader(layer).iterShapeRecords():
            geom=clean(row.shape).simplify(0.008,preserve_topology=True)
            if layer in ['urban','lakes']:
                paths=[list(p.exterior.coords) for p in parts(geom)]
            elif geom.geom_type=='LineString': paths=[list(geom.coords)]
            else: paths=[list(g.coords) for g in getattr(geom,'geoms',[]) if g.geom_type=='LineString']
            for path in paths:
                # Split source lines at the date line instead of spanning the map.
                chunks=[[]]
                for x,y,*_ in path:
                    q=[round(x,5),round(y,5)]
                    if chunks[-1] and abs(chunks[-1][-1][0]-q[0])>180: chunks.append([])
                    chunks[-1].append(q)
                for chunk in chunks:
                    if len(chunk)>1:
                        xs,ys=zip(*chunk)
                        lines.append(dict(kind=layer,points=chunk,bounds=[min(xs),min(ys),max(xs),max(ys)]))
    write(output/'atlas.json',dict(countries=countries,cities=cities,lines=lines))
    source_hashes['terrain.tif']=digest(source/'terrain.tif')
    image=Image.open(source/'terrain.tif').convert('RGB')
    if image.width>21600 or image.height>10800: raise ValueError('Raster resolution exceeds contract')
    tiles=[]
    # Power-of-two 2:1 pyramid; 512px interiors plus a replicated 1px gutter.
    for level in range(5):
        w,h=1024*2**level,512*2**level
        resized=image.resize((w,h),Image.Resampling.LANCZOS)
        for y in range(h//512):
            for x in range(w//512):
                tile=Image.new('RGB',(514,514))
                crop=resized.crop((x*512,y*512,(x+1)*512,(y+1)*512))
                tile.paste(crop,(1,1))
                # Neighbour samples provide seam-safe filtering, including longitude wrap.
                for dx,srcx in [(0,(x*512-1)%w),(513,((x+1)*512)%w)]:
                    tile.paste(resized.crop((srcx,y*512,srcx+1,(y+1)*512)),(dx,1))
                for dy,srcy in [(0,max(0,y*512-1)),(513,min(h-1,(y+1)*512))]:
                    tile.paste(resized.crop((x*512,srcy,(x+1)*512,srcy+1)),(1,dy))
                for dx in [0,513]:
                    for dy in [0,513]:
                        sx=(x*512 + (-1 if dx==0 else 512))%w
                        sy=max(0,min(h-1,y*512 + (-1 if dy==0 else 512)))
                        tile.putpixel((dx,dy),resized.getpixel((sx,sy)))
                name=f'{level}-{x}-{y}.jpg';tile.save(output/name,quality=85,subsampling=0,optimize=False)
                tiles.append(dict(level=level,x=x,y=y,file=name))
    lock=ROOT/'data/map-atlas-sources.json'
    if lock.exists() and json.loads(lock.read_text())!=source_hashes:
        raise ValueError('Source checksums differ from pinned inputs')
    files={p.name:digest(p) for p in sorted(output.iterdir()) if p.name!='manifest.json'}
    write(output/'manifest.json',dict(version=1,vector_revision=VECTOR_REVISION,raster_revision=RASTER_REVISION,
          license='Public domain',license_url='https://www.naturalearthdata.com/about/terms-of-use/',
          limitations='Modern reference geography; not verified for 2000. No simulation authority or transport effects.',
          source_sha256=source_hashes,files=files,tiles=tiles))
    if not lock.exists(): write(lock,source_hashes)
    validate(output)


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--source-dir',type=Path);p.add_argument('--validate',action='store_true');p.add_argument('--check',action='store_true');a=p.parse_args()
    if a.validate: validate(); return
    if not a.source_dir: p.error('--source-dir is required for compilation')
    if a.check:
        with tempfile.TemporaryDirectory() as d:
            build(a.source_dir,Path(d))
            if (Path(d)/'manifest.json').read_bytes()!=(OUT/'manifest.json').read_bytes(): raise ValueError('Non-reproducible atlas')
    else: build(a.source_dir,OUT)

if __name__=='__main__': main()
