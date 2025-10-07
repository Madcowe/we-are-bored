## Installation

Just down load current the current release and run, you will probably need to allow execution
depending on you operating system.

For Linux/android you can do with this command or similar

```bash
chmod774 surf-bored
```

For windows you may get a warning dialog where you need to say run it anyway.

## Arguments

local - to run on local network as [per](https://docs.autonomi.com/developers/how-to-guides/local-network)

## Known issues and limitations

- Changing the terminal size during the working... pop up box will make the rendering go strange.
Once the action it was working on is completed you can quit and restart to fix it.
- May also happen in a number of other situations.
- Occasionally bored and files that do exist on the autonomi network may fail to load/download
usually with a record not found error as work is ongoing to ensure stability...usually resolves with
a few tries.
- Boreds with vary large areas but within the protocol specification may cause very slow response
down to the OS killing the app due to a buffer for the entire area being rendered on each frame.
Boreds of this size are likely to be impractical to use and cannot be created with surf-bored itself.
- The algorithm for picking the next notice in a given direction is idiosyncratic, particularly
with overlapping notices...in most case you will get there eventually but in some cases you may
need to use tab/alt-tab to cycle though them.

### Android

- Doesn't seem to work on Android 9...seems to be an issue with Rust 2024 not working so suspect it
won't work on earlier versions.
