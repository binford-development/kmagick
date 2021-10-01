/// Construct a wand wrapper over Wand types which implements Send
/// and (naturally) deref and deref_mut
macro_rules! wand_wrapper {
    ($name:ident) => {
        use std::ops::{Deref, DerefMut};
        use jni_macros::{jni_class, jni_new, jni_destroy};

        struct $name {
            wand: magick_rust::$name
        }

        unsafe impl Send for $name {}

        impl Deref for $name {
            type Target = magick_rust::$name;

            fn deref(&self) -> &Self::Target {
                &self.wand
            }
        }

        impl DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.wand
            }
        }

        paste::paste! {
            #[jni_class(pkg="com/cherryleafroad/kmagick", exc="com/cherryleafroad.kmagick/" $name "Exception")]
            impl $name {
                #[jni_new]
                fn new() -> Self {
                    Self {
                        wand: magick_rust::$name::new()
                    }
                }

                #[jni_destroy]
                fn destroy() {
                    // object dropped when this scope ends
                }
            }
        }
    }
}
