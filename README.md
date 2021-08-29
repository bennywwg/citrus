A simple and safe Entity Element System framework for rust.

# Structure

#### `trait Element`, `Entity`
There are two core concepts in ecs: the struct Entity, and the trait Element. ecs allows instances of objects that implement Element to be attached to Entities, and safely manages the lifetime of both entities and the attached element instances. These objects are interacted with via the types listed below.

#### `ElementHolder`, `EntityHolder`
These act as strong pointers that wrap a dynamically typed `Element` or `Entity`, respectively. `drop`ing these while any references to these objects exist will panic.

#### `CompononentAddr<T>`, `EntAddr`
These are weak pointers that wrap either an instance of a element or entity, respectively. They become invalid if the wrapped object is dropped, and are then unable to produce references.

#### `ElementRef<T>`, `ElementRefMut<T>`, `EntRef`, `EntRefMut`
These are very similar to the `Ref<T>` and `RefMut<T>` objects returned by a `RefCell`; use them as references to the underlying objects.

#### Diagram
![Diagram](https://raw.githubusercontent.com/bennywwg/ecs/master/diagram.png?raw=true)

#### Manager
TODO!

# Future Plans

There is one flaw in this system that can cause a `panic!`: dropping an `Entity`, for example, while a reference to it is held. And unfortunately, this situation is not unusual and actually comes up very frequently in entity element system codebases. The future implementation of `Manager` intends to address this. Destroying `Element`s and `Entity`s will be prevented from occuring in userspace code. Rather, users will be able to enque an `EntAddr` or `EleAddr<T>` for deletion, which will then be destroyed in a post-pass cycle, during which time it is impossible for references to be held. `Manager` will manage this behavior.

After fixing this issue, it should not be possible for erroneous use of this library to induce `panic!` in any way.
