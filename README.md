# async_buf_reader_utils

The purpose of this crate is to add additional functionality to the [async-std](https://github.com/async-rs/async-std) crate for the async BufReader implementation.

To add the additional methods to async BufReader, simple add this to your given module:

```
use async_buf_reader_utils::prelude::*;
```


# Methods

## buf_reader.read_until_index_found(&mut predicate, &mut fillable).await

```
    fn read_until_index_found<'a, F>(
        &'a mut self,
        predicate_found_index: &'a mut F,
        buf: &'a mut Vec<u8>,
    ) -> ReadChunkUntilIndexFoundFuture<'a, Self, F> 
```

# Example

The goal of this example is to load a text file and asynchronously find 
groups of characters leading up to the 4th comma within the given group.

so if the input text file is:

```
1,50,0,3,17,1,212,3,1,3,4,3,1,5
```

we want to be able to return chunks of:
```
> 1,50,0,3,
> 17,1,212,3,
> 1,3,4,3,
> 1,5
```

Here is the business logic to solve this problem with async read_until_index_found:

```no_run
      fn main() {
        let _result: Result<(), String> = async_std::task::block_on(async {
        
            use async_buf_reader_utils::prelude::*;
            use async_std::{fs::File, io::BufReader};

            // Path to file you want to read asynchronously
            let str_path = "./data/file.txt";

            // Async load file (you can optionally use method with_capacity to limit how 
            // much async-std BufReader bytes are loaded while reading which is useful in memory sensative environments)
            let mut buf = BufReader::with_capacity(12, File::open(&str_path).await.expect("should have opened file"));
            
            let mut comma_count: usize = 0;
            let mut last_comma_idx = None;
            let mut fillable = vec![];

            // Our predicate closure is our business logic,
            // where we are trying to capture all characters up to
            // the 4th comma, remembering that a regex won't be as useful here
            // because our predicate will be called for every loaded chunk
            // of data returned by our async BufReader, so a desired match 
            // might stretch across two different strings provided to the predicate.

            // Once we've reached the 4th comma, return the index via Some(index) and reset
            // our counters, otherwise return None to indicate we are still searching.
            let mut predicate = |str_data: &str|->Option<usize>{
                // println!("in predicate {:?}", str_data);
                last_comma_idx = str_data.bytes().enumerate().find_map(|(i, x)| {
                    if x == b','{
                        comma_count += 1;
                    }
                    if comma_count == 4 {
                        Some(i)
                    }
                    else{
                        None
                    }
                });
                // println!("cur comma idx {:?}", last_comma_idx);
                if let Some(idx) = last_comma_idx {
                    last_comma_idx = None;
                    comma_count = 0;
                    Some(idx)
                }else{
                    None
                }
            };

            // Start our async while loop passing our predicate and buffer to read_until_index_found,
            // so our loop will continue returning the chunks of strings that are resulting
            // from the returned indices in our predicate.
            while let Ok(Some(_)) = buf.read_until_index_found(&mut predicate, &mut fillable).await {
                println!("in while await loop {:?}", str::from_utf8(&fillable).unwrap());
                fillable.clear();
            }
            Ok(())
        });
      }
```