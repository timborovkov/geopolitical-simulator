"""Copy validated offline map assets beside a built desktop executable."""
import argparse
from pathlib import Path
import shutil
from build_map_atlas import OUT, validate

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('executable', type=Path)
    args = parser.parse_args()
    if not args.executable.is_file():
        parser.error('Build the executable before packaging assets')
    validate()
    destination = args.executable.resolve().parent / 'map-atlas'
    shutil.copytree(OUT, destination, dirs_exist_ok=True)
    validate(destination)
    print(f'Packaged offline atlas: {destination}')
