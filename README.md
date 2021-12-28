A simple and safe Entity Component (aka Element) System framework for rust.

![example workflow](https://teamcity.spkit.org/app/rest/builds/affectedProject:name:Tiler2/statusIcon)

# Structure

#### `trait Element`, `Entity`
There are two core concepts in citrus: the struct Entity, and the trait Element. citrus allows instances of objects that implement Element to be attached to Entities, and safely manages the lifetime of both entities and the attached element instances. These objects are interacted with via the types listed below.

#### `ElementHolder`, `EntityHolder`
These act as strong pointers that wrap a dynamically typed `Element` or `Entity`, respectively. `drop`ing these while any references to these objects exist will panic.

#### `EleAddr<T>`, `EntAddr`
These are weak pointers that wrap either an instance of a element or entity, respectively. They become invalid if the wrapped object is dropped, and are then unable to produce references.

#### `EleRef<T>`, `EleRefMut<T>`, `EntRef`, `EntRefMut`
These are very similar to the `Ref<T>` and `RefMut<T>` objects returned by a `RefCell`; use them as references to the underlying objects.

#### Diagram
This diagram visually illustrates the paradigm described above.

![Diagram](https://raw.githubusercontent.com/bennywwg/ecs/master/diagram.png?raw=true)

#### Manager
Manages deferring the destruction of `ElementHolder` and `EntityHolder` so that no references can be held to the underlying objects at the time they are dropped.
