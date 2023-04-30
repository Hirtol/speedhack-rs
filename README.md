# ⚡ Speedhack-rs ⚡

A simple utility to accomplish the same speedhack as can be found in CheatEngine,
without all the additional features (and potential malware 🙃).

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
  "console": false,
  // By default the keys to reload the config are `CTRL + SHIFT + R`
  "reload_config_keys": [
    17,
    16,
    82
  ],
  // Game speed will be set to 10x while `CTRL + SHIFT` are pressed.
  "speed_states": [
    {
      "keys": [
        16,
        17
      ],
      "speed": 10.0,
      "is_toggle": false
    }
  ]
}
```

