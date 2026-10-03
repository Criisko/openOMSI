# Modding beyond OMSI 2

openOMSI reads OMSI 2 content as it is: a bus, a map or an object made for OMSI 2 works
without changes. It also lifts limits OMSI 2 put on modders. Everything on this page is an
addition. A file that uses it still loads in OMSI 2, which ignores what it does not know.

## Interior lights: more than four per mesh

OMSI 2 lights a mesh with at most four `[interiorlight]` lamps, the four numbers of its
`[illumination_interior]`. openOMSI takes as many as you list, up to 63 per mesh. Write the
extra lamp numbers on the lines right after the first four; a blank line ends the list:

```
[mesh]
saloon.o3d

[illumination_interior]
0
1
2
3
4
5
6
7

[matl]
...
```

OMSI 2 reads the first four and skips the rest. A model may declare any number of
`[interiorlight]` lamps, and each mesh names the ones that light it. The same goes for
`[illumination_interior]` in `passengercabin.cfg`.

## Textures of any resolution

- Textures up to 16384 × 16384 pixels load: DDS (DXT1/3/5, uncompressed), TGA, BMP, PNG and
  JPG. Where the graphics card cannot take that size, the texture is halved until it fits.
- There is no 2 GB address-space limit: openOMSI is a 64-bit program.
- A texture keeps its full resolution within 150 m of the camera, so a bus's own 4K
  textures always stay sharp. Far scenery gives up detail only when the texture memory
  (`texture_memory` in the settings, or `OMSI_TEXTURE_MEMORY`, in MB) runs out, as OMSI's
  `[texmemlimit]` does.
- `[scripttexture]`, `[htmltexture]` and `[texttexture]` / `[texttexture_enh]` can be any size the card
  takes.

## PBR materials

Normal, roughness, metalness and occlusion maps beside a texture, up to 4096 × 4096. See
[PBR materials](PBR.md).

## Lights

- Each 25 m square of the world draws up to 32 point and spot lights at once (the nearest
  first), so depots, stations and lit interiors keep their lamps.
- Interior lamps of vehicles are drawn per pixel, with no count limit per vehicle.

## Models

- `.o3d` files with 32-bit indices (the long-index flag) are drawn with their full vertex
  and face count; everything is drawn with 32-bit indices.
- There is no limit on the number of meshes, materials, `[matl_change]` items, `[CTC]`
  entries, cameras, doors, passenger places, wheels or axles.

## Scripts and plugins

- Script variables and string variables have no count limit.
- Lua plugins can read and write script variables, fire triggers and react to game events:
  see [Plugins](PLUGINS.md).

## Radio: a map's stations and a bus's display

What a player sees and hears is in the [user guide](USER_GUIDE.md#radio); here is what a map
or a bus can add. Omsi.exe reads none of it, so a map or bus made for both games loses
nothing in OMSI 2.

**A map's own stations.** `radio.cfg` beside the map's `global.cfg`, written like the
player's (`name = address` a line). Its stations come first on the station buttons while the
map is driven, the player's follow (one with the same address as a map's is left out). Its `volume`
line is not read: loudness stays the player's business. The file is read again when another
map is loaded.

**Frequencies.** Behind the address may stand the frequencies the station is on, for radios
that show one: `| 94.6` is the station's frequency everywhere, `| 94.6 @ x, y` the one near
that place of the map. A station may have as many as it has transmitters along the route:

```
# name = address | frequency @ x, y | ...
Radiozurnal = https://rozhlas.stream/radiozurnal.mp3 | 94.6 @ 25500, 20000 | 90.9 @ 2307, -717
Regional    = https://example.org/regional.mp3 | 97.9
```

`x, y` are the game's metres: the tile's column and row times 300 m plus the place within
the tile. The easiest way to get them is the log: `spawned at entry point 38 "Chlum,hl.sil."
(12810.6, 3735.5, 60.3)` when a bus is put on the map, or an entry point's own numbers in
`global.cfg` (`[entrypoints]`: the place in the tile, then the tile's index into `[map]`).
The place nearest to the bus decides which frequency is shown; there is no blending, so one
place per town along the route is enough. A map's places are map positions, not
coordinates on the globe: many maps shorten and bend their routes.

**A bus's display.** A radio script that wants the station and the song in its display
declares the string variable `Snd_Radio_Text` (in a `[stringvarnamelist]` file of the bus)
and shows it in a text texture: openOMSI writes the text there, ten characters, a longer
text running through, while the radio plays. The variables the radio itself sets are those
of OMSI's radio plugins - `Snd_Radio` (1 while a cassette or the radio plays: the first
station), or `SndExt_Radio` (the station button, 0 = off) with `SndVol_Radio` (the volume,
0..1, up to 2). Dmitrij's "Magnitola" is served as it comes: while it shows its track,
its `magnitola_1` (`frequency@station`, `@` the line break) gets the map's frequency for the
place in its first line and the station and song in its second.

## What stays as in OMSI 2

The following behave as in OMSI 2 so that existing content works unchanged:

- the script stack (8 values) and registers (`l0`-`l9`, `s0`-`s9`);
- one `[spotlight]` lit at a time per vehicle (the one `Spot_Select` picks);
- 100 particles per emitter.

In a LAN session, other players see up to 7 doors, 15 wheels and 127 lamps of a vehicle.
