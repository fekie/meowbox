# Info
This is a work in progress handheld electronic. It uses an esp32s3 as its chip.

The board has been fully constructed, and it is a TODO to add pictures, and potentially schematics to the board.


# Helpful Commands

Basically, there are (for now) only a few commands that are needed to operate and develop this project. 

## Building and Flashing
```bash
cargo run --release
```


## Converting Audio to MP3

This command uses a `.mp3` as input, but will generally work with most audio files.
```bash
ffmpeg -i input.mp3 -f s16le -acodec pcm_s16le -ar 44100 -ac 2 output.pcm
```


