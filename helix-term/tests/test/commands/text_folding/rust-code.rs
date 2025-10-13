//! top-level comment
//! top-level comment
//! top-level comment

mod format_is_needed {

    /// `Bazz` doc comment
    /// `Bazz` doc comment
    /// `Bazz` doc comment
    pub struct Bazz {
g: i32,
            // `b` comment
            // `b` comment
b: i32,
    } // interfering comment

                pub struct Fizz<T, U, V>
    // `where` block comment
    // `where` block comment
    // `where` block comment
where
T: Copy,
U: Copy,
        V: Copy,
    {
        // interfering comment
        a    :     T   ,
        b    :     U   ,
        /// `c` doc comment
        /// `c` doc comment
        /// `c` doc comment
        c: V,
    }
}

/* block comment
block comment
block comment */

/// `TraitA` doc comment
/// `TraitA` doc comment
/// `TraitA` doc comment
/// `TraitA` doc comment
trait TraitA {
    fn f(self, a: u32, b: u32) -> u32;

    fn g(self) -> u32;
}

impl<T, U, V> TraitA for format_is_needed::Fizz<T, U, V>
where
    T: Copy,
    /* block comment
    block comment */
    U: Copy,
    /* block comment
    block comment */
    V: Copy,
{
    fn f(self, a: u32, b: u32) -> u32
/* interfering block comment
     interfering block comment
     interfering block comment */ {
        todo!(
            "
            write some code
            write some code
            write some code
            "
        );
    } // interfering comment
      // interfering comment
      // interfering comment

    fn g(self) -> u32 {
        // comment inside function
        // comment inside function
        // comment inside function

        fn nested() {
            todo!(
                "
                    write some code
                    write some code
                    write some code
                "
            );
        }

        struct Nested /* interfering block comment
        interfering block comment
        interfering block comment */ {
            a: i32,
            b: i32,
            c: i32,
        }

        impl Nested {
            fn h() {
                // really nested comment
                // really nested comment
                // really nested comment

                struct ReallyNested {
                    a: i32,
                    b: i32,
                }
            }
        }

        /* block comment inside function
        block comment inside function
        block comment inside function */
    }
}
