# Virtual memory arena

This crate provides the implementation of a bump memory allocator or memory arena.
The implementation relies on the fact that virtual memory allocation does not need
to be backed by the physical memory page as various OSes provide
the memory management APIs allowing for manual committing the pages to the physical RAM.
