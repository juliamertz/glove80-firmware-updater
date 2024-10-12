# glove80-firmware-updater


A simple command line utility to update the firmware on a [Glove80](https://www.moergo.com/) device.

## Examples

> On linux you can pass --mount to automatically mount devices if your Desktop environment doesn't do this for you. This will prompt you for your password

Run the firmware updater with default values:
```bash
glove80-firmware-updater -f firmware.uf2
```
Run the firmware updater with a full path to the firmware file:
```bash
glove80-firmware-updater -f /home/user/firmware.uf2
```

Run the firmware updater with non-default values:
```bash
glove80-firmware-updater -f firmware.uf2 -l GLV80LHBOOT -r GLV80RHBOOT
```

# Credits

This repo is forked from [https://github.com/jereanon/glove80-firmware-updater](jereanon/glove80-firmware-updater)

License: BSD-2-Clause
