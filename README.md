# homeassistant-mpris-bridge-rust

## Introduction

Linux application to handle control of Home Assistant media players.
A rewrite of the very nice ![repository by rytilahti](https://github.com/rytilahti/homeassistant-mpris-bridge).
If you want it to be fully featured, use that one instead.
This will eventually have feature parity, but it is not there yet.

## Installation

No instructions available right now beyond `git clone` this repository, then `cargo run`. 

## Configuration

Configuration is stored in `~/.config/ha_mpris_bridge/config.toml`. 
This file is created if it does not exist the first time you run the bridge. 
It will also error the first time you run it, as the placeholder values are wrong.
Yes, I will fix the error so it at least exits nicely and explains what happened and how to fix it.

## Missing features

- It currently does not handle volume
- Position seeking seems buggy. Uncertain if that's on me, or on MPRIS 

