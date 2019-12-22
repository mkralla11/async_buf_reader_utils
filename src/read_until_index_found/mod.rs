use async_std::future::Future;
use std::str;
// use std::io::{IoSliceMut, Read as _};
use std::mem;
use std::pin::Pin;

use futures::io::{AsyncRead};


use async_std::{fs::File, task};
use async_std::io::{self, prelude::BufReadExt, BufReader, BufRead};
use async_std::task::{Context, Poll};

pub struct ReadChunkUntilIndexFoundFuture<'a, T: BufRead + ?Sized, F: FnMut (&str) -> Option<usize>> {
    pub(crate) reader: &'a mut T,
    pub(crate) predicate_found_index: &'a mut F,
    pub(crate) buf: &'a mut Vec<u8>,
    pub(crate) read: usize,
}

impl<T: BufRead + Unpin + ?Sized, F: FnMut (&str) -> Option<usize>> Future for ReadChunkUntilIndexFoundFuture<'_, T, F> {
    type Output = io::Result<Option<usize>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Self {
            reader,
            predicate_found_index,
            buf,
            read,
        } = &mut *self;
        read_until_index_found(Pin::new(reader), cx, predicate_found_index, buf, read)
    }
}



pub fn read_until_index_found<R: BufReadExt + ?Sized, F: FnMut (&str) -> Option<usize>>(
    mut reader: Pin<&mut R>,
    cx: &mut Context<'_>,
    predicate_found_index: &mut F,
    buf: &mut Vec<u8>,
    read: &mut usize,
) -> Poll<io::Result<Option<usize>>> {
    loop {
        let min = |x: usize, y: usize| -> usize {
            if x > y {
                y
            } else {
                x
            }
        };
        let (done, used) = {
            let available = futures_core::ready!(reader.as_mut().poll_fill_buf(cx))?;
            if let Some(i) = predicate_found_index(str::from_utf8(&available).unwrap()) {
                if available.len() > 0 {
                    let i_clamped = min(i, available.len() - 1);
                    buf.extend_from_slice(&available[..=i_clamped]);
                    (true, i_clamped + 1)
                } else {
                    (true, 0)
                }
            } else {
                buf.extend_from_slice(available);
                (false, available.len())
            }
        };
        reader.as_mut().consume(used);
        *read += used;
        if used > 0 && done {
            return Poll::Ready(Ok(Some(mem::replace(read, 0))));
        }
        else if used > 0 && !done {
            return Poll::Pending;
        }
        else {
            return Poll::Ready(Ok(None));
        }
    }
}

pub trait ReadUntilIndexFound: BufRead {
    fn read_until_index_found<'a, F>(
        &'a mut self,
        predicate_found_index: &'a mut F,
        buf: &'a mut Vec<u8>,
    ) -> ReadChunkUntilIndexFoundFuture<'a, Self, F>
    where
    F: FnMut (&str) -> Option<usize>;
}

    #[doc = r#"
        Allows reading from a async buffered byte stream.
        Include all of async buffer utils methods by including the [prelude]:
        ```
        # #[allow(unused_imports)]
        use async_buf_reader_utils::prelude::*;
        ```
        Or you can include this given util self method individually:
        ```
        # #[allow(unused_imports)]
        use async_buf_reader_utils::read_until_index_found::*;
        ```
        [prelude]: async_buf_reader_utils::prelude

    "#]
impl<T> ReadUntilIndexFound for BufReader<T> where T: AsyncRead{

        #[doc = r#"
        Reads all bytes into buf until the result of calling the provided 
        predicate_found_index closure returns Some(found_index: i32). 
        If predicate_found_index returns None, predicate_found_index will 
        continue to be called repeatedly until Some(found_index: i32) 
        or EOF is reached.

        This function will read bytes from the underlying stream until the 
        predicate_found_index is Some(found_index: i32) or EOF is found. 
        Once found, all bytes up to, and including, the delimiter (if found) 
        will be appended to buf.

        If successful, this function will return the total number of bytes read.

        # Examples

```no_run
        #let _result: Result<(), String> = async_std::task::block_on(async {
        #
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
        #    Ok(())
        #});
```
    "#]
    fn read_until_index_found<'a, F>(
        &'a mut self,
        predicate_found_index: &'a mut F,
        buf: &'a mut Vec<u8>,
    ) -> ReadChunkUntilIndexFoundFuture<'a, Self, F> 
     where
     F: FnMut (&str) -> Option<usize>{
         ReadChunkUntilIndexFoundFuture{
            reader: self,
            predicate_found_index,
            buf,
            read: 0
        }

    }
}





#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_file_as_buffer_providing_strings_to_predicate_filling_buffer_with_respective_data_when_index_is_returned_until_eof() {
        let _result: Result<(), String> = task::block_on(async {
            let str_path = "./data/file.txt";
            let mut buf = BufReader::with_capacity(12, File::open(&str_path).await.expect("should have opened file"));
            let mut comma_count: usize = 0;
            let mut last_comma_idx = None;
            let mut fillable = vec![];
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

            while let Ok(Some(_)) = buf.read_until_index_found(&mut predicate, &mut fillable).await {
                // println!("in while await loop {:?}", str::from_utf8(&fillable).unwrap());
                fillable.clear();
            }
            Ok(())
        });
    }
}
