# JPEG Steganography Project

## Overview

This project is the result of a work hackathon, to use steganography to write a message into a JPEG image, by re-ordering one of the huffman tables.

It was primarily written as a self-educational tool.

---

## Building

This is written in stable Rust, version 1.62.0. A `cargo build` should be fine to build it for debug, and `cargo build --release` for production.

---

## Running

Typical usage will consist of one of three methods:

* Take an existing JPEG file, create a new file with an encoded secret string
  > cargo run -- <*my-input-file*> write <*my-output-file*> <*my-secret-string*>

* Read a secret string from a JPEG file
  > cargo run -- <*my-input-file*> read

* Show debug information about the various marker segments in a file
  > cargo run -- <*my-input-file*>

---

## Limitations

This was written just for bog-standard JPEG files. Progressive JPEGs aren't supported and files that are heavily optimized tend to have a much smaller set of huffman table sizes to choose from, and will be limited in the size of the encodable secret.

The default tables (I think?) will yield about 88 to 92 bytes of space for their AC huffman tables, whereas you might get lucky to have 18 bytes of space within a properly optimized JPEG file.

I don't make any promises on the functionality of the program, but it shouldn't blow anything up.

## Explanation

### Permutation

Given a list of N elements, there are N! (factorial) ways to uniquely re-order the elements.

For example, with 3 elements, there are 3! or 6 ways to permute the elements. Lexically sorted, they are as follows:

| Index | Permutation |
| - | - |
| 0 | 1 2 3 |
| 1 | 1 3 2 |
| 2 | 2 1 3 |
| 3 | 2 3 1 |
| 4 | 3 1 2 |
| 5 | 3 2 1 |

To convert from the numerical index to the permutation, you convert the number to its [factorial numbering system](https://www.wikiwand.com/en/Factorial_number_system). This is the unique way of representing the number as a series of coefficients multiplied by the increasing factorials.

For example, the value 4 is uniquely represented as 2 \* 2! + 0 \* 1!. Otherwise represented as \[2; 0]<sub>!</sub>

The numbers 0 through 5 can be represented as (using two digits):

| Index | FNS |
| - | - |
| 0 | \[0; 0]<sub>!</sub> |
| 1 | \[0; 1]<sub>!</sub> |
| 2 | \[1; 0]<sub>!</sub> |
| 3 | \[1; 1]<sub>!</sub> |
| 4 | \[2; 0]<sub>!</sub> |
| 5 | \[2; 1]<sub>!</sub> |

To obtain a permutation of the elements, you simple create a sorted array of each number to serve as a working set. Then, for each digit N of your FNS representation, you remove the Nth (base zero) item from the working set. You'll end up with one final value, which is appended to the end. For example, using 4 = \[2; 0]<sub>!</sub>:

| Digit | Working Set | Permutation |
| - | - | - |
| 2 | 1 2 __3__ | 3 |
| 0 | __1__ 2 | 3 1 |
| - | __2__ | 3 1 2 |

Thus, by using the factorial number system you can turn a number to a permutation and vice-versa.

### The JPEG file format

The JPEG file format consists of two-byte markers for a segment of data. For example, there's the SOF\[n] marker that signifies the frame data for the jpeg, e.g. the width & height.

A simple marker segment might look like:
| Bytes (Hex) | Meaning |
| - | - |
| \[0xFF 0xDD] | DRI (Define Restart Interval) |
| \[0x00 0x04] | Segment is 4 bytes long |
| \[0x00 0x60] | Restart Interval of 96 (=0x60) MCUs |

There's also the entropy stream, that lies within the SOS (Start of Scan) segment, but that's an entirely different kettle of fish.

### DHT Segments

The DHT segments define huffman tables for JPEG. They consist of, mainly, two arrays: symbol sizes and symbol values.

The sizes array can be used to calculate the huffman symbols for the table. The manner in which that is done, is beyond the scope of this document, but it turns out it's surprisingly simple. Here's an example small huffman table for:

> Size: \[0, 1, 5, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0]  
  Values: \[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]

__Huffman Table:__
| Value | Bits |
| - | - |
| 0 | 00 |
| __1__ | 010 |
| __2__ | 011 |
| __3__ | 100 |
| __4__ | 101 |
| __5__ | 110 |
| 6 | 1110 |
| 7 | 11110 |
| 8 | 111110 |
| 9 | 1111110 |
| 10 | 11111110 |
| 11 | 111111110 |

However, notice that we can change the order of the entries 1 through 5, and not seriously impact the size of the output jpeg file, because they're all the same length! This means, if we permutate the order of the identically-sized segments, we can store a secret number in the ordering. To read out the secret, we merely have to look at the ordering, extract out a number from it, and see if it corresponds to a secret message.

### Oh... and the entropy stream

If we just leave it there, we're going to destroy the image, because it's been encoded with the old huffman table. The solution, therefore, is to write a minimal entropy stream reader, read in the old huffman table's symbols and write out the new table's encoding for it.

### Conclusion

This was a fun project. I got to write a parser for JPEG files, play around with permutation math, and have fun with the huffman tables. What more could one ask for?

---

## Example

Here is a picture of a mourning dove:

![A nesting mourning dove](/docs/dove-small-in.jpg "A nesting mourning dove")

And here is the same image, but with an encoded secret message inside: [dove-small-out.jpg](/docs/dove-small-out.jpg)

Comparing the two files, yields a small difference in file size:

| File | File Size|
|-|-|
| dove-small-in.jpg | 80320 |
| dove-small-out.jpg | 80324 |

The pixels, however, are identical. The only difference between the two files, is the order of the Huffman tables inside (and the stream, rewritten using the new table). The encoded secret is a simple lorem ipsum:

"Lorem ipsum dolor sit amet, consectetur adipiscing elit. Nullam vel convallis ipsum. Ut sed ipsum diam. Nam mattis semper iaculis. Nam in dui eu erat aliquam dapibus."

One should be able to verify this with:
> cargo run -- docs/dove-small-out.jpg read

Output:
> Secret: 'Lorem ipsum dolor sit amet, consectetur adipiscing elit. Nullam vel convallis ipsum. Ut sed ipsum diam. Nam mattis semper iaculis. Nam in dui eu erat aliquam dapibus.'
