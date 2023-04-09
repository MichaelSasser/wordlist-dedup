# wordlist-dedup

wordlist-dedup is a program written in rust to deduplicate wordlists. Duh.

I've tried to deduplicate lines of a huge wordlist (>80 GB) with GNU/coreutils
`uniq`. First everything seemed to be hunky dory. Before I deleted the original
I checked the size difference between the two files. It was about half of the
original and not just about 5% less, which I initially suspected.

To confirm this, I wrote a program to count the duplicates and Bingo! The
original file had just a smidgen over 3 % of duplicates.

Maybe I did something wrong or my PC was not able to handle the memory
requirements of `uniq`. I don't know, why it needs that much memory and has a
super low throughput. The wordlist must be sorted before using `uniq` anyway.
Everything uniq has to do is compare line `n` with line `n-1`. If they are both
the same, don't write one of them to another file. That simple.

So, here is wordlist-dedup. It does exactly that. On my old laptop it writes
roughly 400 MB/s of deduplicated output. When only counting the number of
duplicates, it reads about 1.4 GB/s of data. Both with a line length of 200
utf-8 characters.

I use it together with a script, that first runs GNU/coreutils `sort` (which
works as intended) and `wordlist-dedup` after that.

## Command line tool

```commandline
$ wordlist-dedup --help
Deduplicate Pre-Sorted Wordlists

Usage: wordlist-dedup [OPTIONS] <SRC> [DEST]

Arguments:
  <SRC>   The pre-sorted source file, wich may contains duplicated lines
  [DEST]  The destination file, to write the deduplicated file to

Options:
  -c, --count    Only count the number of duplicates
  -h, --help     Print help
  -V, --version  Print version
```

Please keep in mind, the file must be sorted beforehand. Otherwise, it may not
find duplicates. You can use GNU/coreutils `sort`, which does a fine job, even,
when the file is larger than the available memory. wordlist-dedup uses what it
gets of memory. If there is a bunch of memory available, it might use it (to an extend). If there is barely any memory left, it will work as well. Just slower.

You can use it to deduplicate a file like:

```commandline
$ wordlist-dedup some_file_with_dups.txt new_file_to_write_to.txt
  ⠏ Checking for duplicates... (0.8 s)
  🚀 Found 410 duplicates in 2.1 s.
```

During operation it outputs a spinner in the front, to signal, that
that the program is still running. When it is done, it tells you the number of
duplicate line.

If you use it with only one argument like `file.ext`, it names the destination
file `file_uniq.ext`.

Keep in mind, it was made for one job, to deduplicate sorted wordlists. It
might work in different scenarios.

If you like to use my scripts to dedup as many files in one folder structure
check out my
[Gist](https://gist.github.com/MichaelSasser/631f297e60f2d2a6cb6d76dfde12e6e4).

## Installation

Make sure you have a recent version of a Rust compiler installed.
To build it, run: `cargo build --release`.

The binary is now located in the "target" folder, which was created while
building the program:
`target/release/wordlist-dedup`.

If you are using a GNU/Linux based operating system, you can also use the
already build binary by the CD workflow. Head over to the
[release tab](https://github.com/MichaelSasser/wordlist-dedup/releases).

## Semantic Versioning

This repository uses [SemVer](https://semver.org/) for its release cycle.

## Branching Model

This repository uses the
[git-flow](https://danielkummer.github.io/git-flow-cheatsheet/index.html)
branching model by [Vincent Driessen](https://nvie.com/about/). It has two
branches with infinite lifetime:

- [master](https://github.com/MichaelSasser/wordlist-dedup/tree/master)
- [develop](https://github.com/MichaelSasser/wordlist-dedup/tree/develop)

The master branch gets updated on every release. The develop branch is the
merging branch.

## License

Copyright &copy; 2020-2023 Michael Sasser <Info@MichaelSasser.org>. Released
under the GPLv3 license.
