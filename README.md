# rhyolite-fluids
An in-progress, bare-bones rendering engine built with Vulkano, tailored for my research project, _Improving sphere blending performance for fluid simulation applications using ray-marched rendering._ Includes separate ray-marched and mesh rendering engines, utilizing marching-cubes and a smooth-min distance field, respectively, for sphere blending

Rhyolite is built first and foremost as a personal project and a renderer for my research, but it can be used for all sorts of 
applications! Currently, it's able to load and render .obj files, both unlit and lit. Unlit models are rendered entirely with a chosen albedo color,
while lit models are shaded with Phong shading. To light models, Rhyolite offers point and directional lights, each with controllable color and 
brightness parameters.

Here's an example of Rhyolite in action:

![rhyolite](https://user-images.githubusercontent.com/29758429/210491738-b8defba2-e8f9-419f-a428-a89a1e326a55.gif)

In order to use Rhyolite, it's necessary to enable the features that you want to use in your `Cargo.toml`. To enable
mesh rendering, do so under `[dependencies]`, as seen below:

```
rhyolite = { version = foo, features = ["mesh"] }
```

and for ray-marched rendering:

```
rhyolite = { version = foo, features = ["marched"] }
```

As of now, ray-marched rendering is unstable and does not feature support for custom functions, so it's not recommended to use. These feature flags, therefore, will avoid bloating binaries with unnecessary code. 

---

The code of this library is based partly on [Taidaesal's Vulkano tutorial](https://github.com/taidaesal/vulkano_tutorial), which, in turn, is based 
on the official Vulkano examples. It's adapted to be more extensible and user-friendly than the code in the tutorial, with Phong-shaded point lights,
more abstraction for `Camera`s and `Renderer`s, performance optimizations, and (a lot of) refactoring for better readability and usability. 

All code used from outside sources has been provided under the [MIT License](https://opensource.org/licenses/MIT).
