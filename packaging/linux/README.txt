Woodworking - 3D layout tool for kitchens and cabinets
=======================================================

For 64-bit Intel/AMD Linux (x86_64).

Install
-------
    tar -xzf woodworking-linux-x86_64.tar.gz
    cd woodworking-linux-x86_64
    ./install.sh

Then start "Woodworking" from the app menu, or run `woodworking` in a terminal.

Your projects are saved in ~/Woodworking. The app opens silvius.json from
there at startup and shows an example kitchen if it is not there yet.

Without installing
------------------
    ./woodworking

Uninstall
---------
    ./install.sh --uninstall      (your projects in ~/Woodworking are kept)

If it does not start
--------------------
Run `woodworking` from a terminal to see the error. The usual cause is a
missing graphics driver: the app needs Vulkan (or OpenGL). On Ubuntu/Debian:

    sudo apt install mesa-vulkan-drivers libasound2 libudev1 libxkbcommon0 libgtk-3-0
