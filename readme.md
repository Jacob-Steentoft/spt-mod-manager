# Single Player Tarkov Mod Manager
A mod manager aiming to provide a smooth experience 
adding, updating, configuring, and removing mods from SPT.

This project was intended for use for friends, but is now being 
opened up for others to use as well. Limited support can be
requested through GitHub issues.

## How to use

To be created.

## Goal
Providing an easy method for interfacing with mods for Tarkov written in Rust.

This can be through local or hosted solutions to improve adoption of the mod scene for Tarkov.

## Features
These are the current features that the application offers. More will be added the
future as more effort is poured into the application.

* Support for downloading mods from 2 remote repositories
  * SPT-Tarkov
  * GitHub
* Simple mod profile
  * Stored and edited in a json file
* Mod installation 
  * Using differential based on last cached install, allowing
for configurations between updates. New updates will overwrite 
local changes.

## Roadmap
My big TODO list of things I want to do for the application. Note that this list
can change whenever I feel like it and development will be slow as I'm likely busy 
playing Tarkov after work :)

* UI
    * This is currently being written with Iced to support the above features. 
* Configuration profiles
    * Allowing users to switch easily between sets of configurations and mods.
* Install configurations
    * Allowing users to make overwrites to how mods are configured between installations and updates. 
* Support for local mod repositories
    * Allowing locally stored and hosted mods to be installed together with remote ones
* More mod remote download support
* Server app for server hosts
  * Allowing hosters to manage their mods remotely
  * Allowing users to install mods before connecting to servers
* API for interfacing with a cached version of SPT-Tarkov
  * APT-Tarkov is currently under stress from traffic so a simpler cached API 
will provide a smoother update experience

## Contributions
Feel free to submit pull requests and issues for feature 
requests. This is a hobby project so expect some delay in response,
but I should hopefully get around to responding.

### Requirements
* Must be written in Rust.
  * For the sake of simplicity and maintainability all contributions should be
using Rust as much as possible.
* Must aid in the goal of making a better experience for consuming mods.
* Should be testable.
  * While not all code is currently testable, PRs are easier to review when you can use
assertions

### Build Dependencies
Currently external dependencies are necessary for compiling the library for handling
extractions from archive files.

### vcpkg
```PS
git clone https://github.com/microsoft/vcpkg.git
cd vcpkg; .\bootstrap-vcpkg.bat
```

### libarchive
```PS
vcpkg install libarchive:x64-windows-static-md
```