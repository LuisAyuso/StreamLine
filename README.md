# StreamLine:
A rust library for easy and efficient 2D drawing. 

StreamLine aims to provide a easy imperative drawing API which can be efficiently rendered. As efficient as we can get it to be :D

## Architecture:

StreamLine is Platform agnostic, for this reason a backend layer needs to be implemented. Currently an OpenGL backend is available, but having a Vulkan or Gfx backedn is feasible and in the roadmap.

The window and event management is currently delegated into the backend but having our own implementation is not discarded.


  


