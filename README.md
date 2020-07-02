# wordlist-dedup

wordlist-dedup is a program written in rust to deduplicate wordlists. Duh.

I tried to deduplicate lines of a huge wordlist (>80 GB) with GNU/coreutils 
`uniq`. First everything seemed to be hunky dory. Before I deleted the original
file I spotted the size of the deduplicated. It was about half of the original.
In the firsthand I suspected about 5 % duplicates duplicates.

To check this, I wrote a program to count the duplicates and Bingo! The
original file had just a smidgen over 3 % of duplicates.

Maybe I did something wrong or my PC was not able to handle the memory 
consumption of uniq. I don't know, why it needs that much memory and is so 
slow. The wordlist must be sorted before using uniq. Everything uniq has to
do is compare line n with line n-1. If they are both the same, don't write the
line n to another file. So easy.

So, here is wordlist-dedup. It does exactly that. I used it with a script, that
first runs GNU/coreutils `sort` first (which works as intended) and then 
wordlist-dedup.


## Command line tool

wordlist-dedup as a pure commandline tool. Keep in mind, the file must be 
sorted before running it. You can use GNU/coreutils `sort`, which does a fine
job, even, when the RAM is limited. This means, the file can be larger then
the available RAM. wordlist-dedup does barely use any RAM.
You can use it to deduplicate a file like:

```
$ wordlist-dedup some_file_with_dups.txt some_file_without_dups.txt
⠏ Done. Found 410 duplicates.
```

During operation it outputs a NPM like spinner in the front, to signal, that
that the program is still running. When it is done, it tells you the number of
duplicate line.

If you use it with only one argument like `file.ext`, it will name the 
outputfile `file_uniq.ext`.

Keep in mind, it was made for one job, to sort wordlists. It might work in 
other scenarios.

## Installation

Just run ``cargo build --release``.

The binary will be stored in the "target" folder:
`target/release/wordlist-dedup`.

If you are using a GNU/Linux based operating system, you can also use the
already build binary by the CD workflow. Head over to the 
[release tab](https://github.com/MichaelSasser/wordlist-dedup/releases).

## Semantic Versioning

This repository uses [SemVer](https://semver.org/) for its release cycle.

## Branching Model

This repository uses the
[git-flow](https://danielkummer.github.io/git-flow-cheatsheet/index.html)
branching model by [Vincent Driessen](https://nvie.com/about/).
It has two branches with infinite lifetime:

* [master](https://github.com/MichaelSasser/matrixctl/tree/master)
* [develop](https://github.com/MichaelSasser/matrixctl/tree/develop)

The master branch gets updated on every release. The develop branch is the
merging branch.

## License
Copyright &copy; 2020 Michael Sasser <Info@MichaelSasser.org>. Released under
the GPLv3 license.
