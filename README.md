A simple and safe Entity Component System framework for rust.

# Structure

#### `trait Component`, `Entity`
There are two core concepts in ecs: the struct Entity, and the trait Component. ecs allows instances of objects that implement Component to be attached to Entities, and safely manages the lifetime of both entities and the attached component instances. These objects are interacted with via the types listed below.

#### `ComponentHolder`, `EntityHolder`
These act as strong pointers that wrap a dynamically typed `Component` or `Entity`, respectively. `drop`ing these while any references to these objects exist will panic.

#### `CompononentAddr<T>`, `EntAddr`
These are weak pointers that wrap either an instance of a component or entity, respectively. They become invalid if the wrapped object is dropped, and are then unable to produce references.

#### `ComponentRef<T>`, `ComponentRefMut<T>`, `EntRef`, `EntRefMut`
These are very similar to the `Ref<T>` and `RefMut<T>` objects returned by a `RefCell`; use them as references to the underlying objects.

#### Manager
TODO!

# Future Plans

There is one flaw in this system that can cause a `panic!`: dropping an `Entity`, for example, while a reference to it is held. And unfortunately, this situation is not unusual and actually comes up very frequently in entity component system codebases. The future implementation of `Manager` intends to address this. Destroying `Component`s and `Entity`s will be prevented from occuring in userspace code. Rather, users will be able to enque an `EntAddr` or `ComponentAddr<T>` for deletion, which will then be destroyed in a post-pass cycle, during which time it is impossible for references to be held. `Manager` will manage this behavior.

After fixing this issue, it should not be possible for erroneous use of this library to induce `panic!` in any way.
