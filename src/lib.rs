//! # async-std BufReader additional functionality
//! 
//! Adds additional functionality to the [async-std](https://github.com/async-rs/async-std) crate for the async BufReader implementation.
//! 

pub mod read_until_index_found; 

    #[doc = r#"
      The async BufReader utils prelude.

      The purpose of this module is to alleviate imports for all of the async BufReader utils traits
      by adding a glob import to the top of heavy modules:

      ```
      # #![allow(unused_imports)]
      use async_buf_reader_utils::prelude::*;
     ```

    "#]
pub mod prelude {
  #[doc(no_inline)]
  pub use crate::read_until_index_found::*;
}