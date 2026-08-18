# zypi

## Zypper patch infos
A small desktop viewer for patches `zypper` knows about on openSUSE and SLE
systems. zypi only reads information from the system it never installs, removes or refreshes anything.

It runs `zypper --xmlout lp` in the background and
displays the current information so everything zypper reports about a
patch is available. This works as an ordinary user no root privileges are needed and none are requested.
Fields that zypper omits are shown as `Unknown`, `Uncategorized` or
`Unspecified` rather than being left blank.
Patches with status `needed` are highlighted in red and appear on top of the patch list.



### UI description

The window is split into a toolbar, a patch list on the left and a details pane
on the right.

**Toolbar**
 
TODO

**Patch list**

TODO

**Details pane**

TODO

### Build

```sh
cargo build --release
./target/release/zypi
```

To run the  XML deserialization test use `cargo test` 

Rust **1.95 or newer** is required due to egui/eframe 0.36 minimum supported version.


### Runtime Requirements


- The `zypper` command in `PATH`. This should be installed by default on  openSUSE or SLE system. 
- Meaningful results need up to date zypper repository metadata. Refreshing needs root,
  so run `sudo zypper refresh` occasionally. No refresh of the metadata is done by zypi.
- A graphical session, either Wayland or X11.

