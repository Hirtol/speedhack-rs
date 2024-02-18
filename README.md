# ⚡ Speedhack-rs ⚡

A simple utility to accomplish the same speedhack as can be found in CheatEngine,
without all the additional bloat.

## How to use
* Download the relevant zip for the game (some older games are x86, modern games usually require x64).
* Extract the `version.dll` and place it next to the game's exe.
  * (Alternatively, a different loader/injector can load this dll just fine as well, feel free to rename it if need be)
* Run the game once, a config file called `speedhack_config.json` will be created in the same directory you placed the `version.dll`
* Edit config to your liking, save, and run the game. (Virtual keycodes can be found [here](https://learn.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes))

## Config

The config will look like this by default:

```json5
{
  // Show a console for debug logging.
  "console": false,
  // By default the keys to reload the config are `CTRL + SHIFT + R`
  "reload_config_keys": [
    "VK_CONTROL",
    "VK_SHIFT",
    "VK_R"
  ],
  // Some games will crash if certain functions are hooked instantly
  // this can introduce a variable delay as needed.
  "wait_with_hook": {
    "secs": 0,
    "nanos": 250000000
  },
  // Skip through opening animations on game launch if desired.
  // Will automatically start & stop a speedup after the given duration at game startup
  "startup_state": {
    "speed": 10.0,
    "duration": {
      "secs": 5,
      "nanos": 0
    }
  },
  // Game speed will be set to 10x while `CTRL + SHIFT` are pressed.
  "speed_states": [
    {
      "keys": [
        "VK_CONTROL",
        "VK_SHIFT",
      ],
      "speed": 10.0,
      "is_toggle": false
    }
  ]
}
```

