# rhyolite
An in-progress, bare-bones rendering engine built with Vulkano.

Rhyolite is built first and foremost as a personal project and a renderer for my fluid simulation research, but it can be used for all sorts of 
applications! Currently, it's able load and render .obj files, both unlit and lit. Unlit models are rendered entirely with a chosen albedo color,
while lit models are shaded with Phong shading. To light models, Rhyolite offers point and directional lights, each with controllable color and 
brightness parameters.

Here's an example of Rhyolite in action:
![rhyolite](https://user-images.githubusercontent.com/29758429/210491134-02861d8e-baa8-4556-aa7f-cb74b0be866e.gif)

The base code here is heavily based on [Taidaesal's Vulkano tutorial](https://github.com/taidaesal/vulkano_tutorial), which, in turn, is based on the 
official Vulkano examples. It's adapted to be more extensible and user-friendly than the code in the tutorial, with Phong-shaded point lights,
more abstraction for `Camera`s and `Renderer`s, performance optimizations, and (a lot of) refactoring for better readability and usability. 

All code used from outside sources has been provided under the [MIT License](https://opensource.org/licenses/MIT).
