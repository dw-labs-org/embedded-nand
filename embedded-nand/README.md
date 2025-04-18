An attempt to create a NAND equialent of the NOR traits in [embedded-storage](https://github.com/rust-embedded-community/embedded-storage).

The aim is to give a target for a flash translation layer / bad block management algorithm (e.g flashmap in this repo) or a filesystem. This means being able to read/write/copy pages or sectors (sub-pages) at a time and erasing blocks.

Compared to the NOR traits, there is a single read and write trait (read only doesn't make much sense, even in read only applications pages can fail / may need refreshing). There are also specific functions for block status checking, marking bad and erasing, and copying data.