# duration-calculator-rs

`duration-calculator-rs` is a command-line utility for parsing duration strings and add and subtract the durations. Durations can be passed via stdin and/or arguments.

## Compile

```sh
cargo build --release
```

You can also install it with
```sh
cargo install --path .
```

## Usage

To use `duration-calculator-rs`, run the compiled binary with the desired duration string as an argument and/or provide the duration string via standard input:
  
```fish
$ duration-calculator-rs "3d 20h 10m 15s"
92h 10m 15s

$ echo -e "2d 5h\n-20m" | duration-calculator-rs
52h 40m 00s

$ echo -e "2d 5h\n-20m" | duration-calculator-rs 23m - 15s
52h 40m 00s
53h 02m 45s
```
  
Note that when using both, an intermediate result is displayed for stdin and then the total result for both stdin and arguments.
  
Please note also, signum is for the whole composite duration, so e.g. `-5m 20s` really means `-(5m20s)`. This might become clearer from the use case description.

## Why?

The reason for me to write it was to measure durations throughout the day and calculate the total duration by adding or subtracting the individual durations. Sure, I could use Excel or Libreoffice Calc and they certainly do duration calculations well enough. But I often record the duration from my phone or tablet on the go to a cloud synced md file and then add everything the next day on my computer. I have not really found anything on the command line that did the overflows between the units in a way that suited me, so I felt I had to come up with something myself.  
  
I'm aware of Google Docs, M365 and Collabora and depending on the situation, I'm using all of them. But that just felt too heavy for this use case. ;)
  
For convenience, I'm using this in my `.vimrc`:

```vim
" pipe selection through command
function! PipeThroughCommand(commandname, ...) range
    let l:args=get(a:, 1, "")
    let l:cmdline = 'echo -e '.shellescape(join(getline(a:firstline, a:lastline), '\n')).' | '.a:commandname ." " . l:args
    let l:output = systemlist(l:cmdline)
    call append(a:lastline, l:output)
endfunction

com! -range -nargs=? DurationCalc :<line1>,<line2>call PipeThroughCommand("duration-calculator-rs", "<args>")
```
  
This allows me to visually select a text block containing durations, press colon and enter DurationCalc as a command. Optionally I can pass last day's total duration as the argument.

A sample text file may look like this:

```md
t 21h10m13s # duration for Monday

Tuesday
--
24h
-15m30s # Break 1
-20m20s # Break 2
-35m30s # Lunch Break
-20m10s # Dinner break
```
  
Now I mark the last 5 lines, use `:'<,'>DurationCalc +21h10m13s` and I get
```
22h 28m 30s
43h 38m 43s
```

![Use case demonstration](/img/output1.gif?raw=true)
(Please note I have changed the padding since the demo.)

p.S.: I'm not really using this for work, although the example might suggest otherwise. For work I'm happy to recommend [Timesheet](https://timesheet.io/). ;)

## Disclaimer

I got some help from ChatGPT to add the annotations and tests. Should they not be correct I might notice at some point. ;) Oh, and parts of the readme.
