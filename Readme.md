# IPL-Checksum GPU Hasher

N64 IPL Hash Brute-forcer using [wgpu](https://github.com/gfx-rs/wgpu) + compute shaders.<br>
> **Note:**<br>
> This assumes the `CIC 6105/7105` with the checksum `8618 A45B C2D3`.<br>
> You should set the seed to `0x9191` when using this tool.

## Usage
Run with a given seed and ROM to start brute-forcing:<br>

```sh
ipl_gpu --seed 0x9191 /path/to/input_rom.z64
```
If a hash was found, a new file with the extension `.match.z64` will be created 
in the same directory as the input file.

Note that this tool is a fully self-contained binary, so you can move it anywhere.

## Build
This project can be built with cargo:
```sh
cargo build --release
```
The resulting binary will be located at `./target/release/ipl_gpu`.

## License
This project is licensed under the MIT License - see [LICENSE](LICENSE)<br>
© 2023 - Max Bebök (HailToDodongo)