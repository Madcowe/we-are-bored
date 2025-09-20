# We are bored protocol

Version: 2

## Abstract

The we are [bored](bored.acronyms) protocol is a minimalist ephemeral social interaction network.
It is implemented using the [autonomi network](https://autonomi.com) and based on the [SMILE](SMILE_Philosophy.md) 
philosophy though falls short of many of those principles.

It's inspiration is real world physical pin/notice boards. It is made up of boreds which are two
dimensional volumes that can contain notices that can contain text which may contain hyperlinks that
can be to other boreds in the network. These may be viewed by anyone who has the bored's address and
as of version 2 anyone with access may also add a notice.

## Definitions

### Coordinates

All coordinates begin at the top left.

### Boreds

- A bored is a named two dimensional volume it contains a collection of notices that are positioned
on the bored, all notices must sit entirely within the bored. All notices must be at least partially
visible.
- As new notices are added they would be considered to be on top of any existing notices that occupy
the same space on the bored.
- It is possible view to the entirety of a notice that parts of which are occluded by subsequent
notices.
- When a notice becomes entirely occluded by subsequent notices it no longer exists.
- The x and y dimensions of the bored must be whole numbers between 0 and 65535 (ie an unsigned 16
bit integer). Each representing a *character of text.
- The bored will be stored in an autonomi (scratchpad)[https://docs.rs/autonomi/latest/autonomi/struct.Scratchpad.html] data type, hence the overall size of the
entire bored cannot exceed the limit of this (4mb).
- If a bored is in excess of the size of a scratchpad it can be reduced by removing the oldest (ie
first in collection) notice and so on until enough space is available.

### Character

In the context of this specification a character is a (Unicode scalar value)[https://www.unicode.org/glossary/#unicode_scalar_value].

### Notices

- A notice is a rectangle that has a capacity to hold text of it's volume minus 1 character at the
edges which is used as a border. Hence a three by three notice would only have 1 character capacity.
- A notices capacity does not include the non-visible element of a hyperlink at render time.
- A notice may contain as many hyperlinks as characters capacity.
- A notice cannot be edited once placed on the bored.

### Hyperlinks

- Hyperlinks are represented in the text of a notice using markdown notation \[Text](URL).
- The brackets and URL of the hyperlink are not rendered, and not included in calculating a notices
text capacity.
- Therefore reasonable limits should be imposed on the length of the URL text, as from a user
perspective it is invisible, so as to not make notice to large to hold in a bored.
- A hyperlink may include a bored URL and if activated in a client application that app should
attempt to move to the bored at that address. It may also include links to other protocols and
the app may choose to handle them as well.

## Data Formats

A notice has the following fields:

- Dimensions: the dimensions of the notice, taken as the coordinate of the bottom right of the
rectangle, the top left is assumed at coordinate (0, 0).
- top_left: the coordinates of the top left of the notice with respect to the bored it is place on.
- content: the characters of the text of the notice plus any additional for representing hyperlinks.

A bored has the following fields:

- Name: Text represent the name of the bored.
- Dimensions: the dimensions of the bored, taken as the coordinate of the bottom right of the
rectangle, the top left is assumed at coordinate (0, 0).
- Notices: A collection of notices.

The bored data is stored in JSON format before being encrypted for storage in a scratchpad.
An example in JSON format is below:
Data: {"protocol_version":1,"name":"The genesis bored","dimensions":{"x":120,"y":40},"notices":[{"top_left":{"x":6,"y":2},"dimensions":{"x":30,"y":8},"content":"Hello is this bored for fans\nof genesis?"},{"top_left":{"x":17,"y":7},"dimensions":{"x":30,"y":10},"content":"No it's just the first bored\nto ever exist."},{"top_left":{"x":55,"y":3},"dimensions":{"x":20,"y":6},"content":"Kuato lives\n"},{"top_left":{"x":54,"y":11},"dimensions":{"x":28,"y":8},"content":"--MY FAVOURITE FONT--\nWhat is your porpoise\n"},{"top_left":{"x":77,"y":3},"dimensions":{"x":28,"y":8},"content":"Welcome to CBBS. Please leave a message or check the latest upload from the CACHE user group.\n"},{"top_left":{"x":71,"y":25},"dimensions":{"x":30,"y":10},"content":"This [link won't work](not a link)"}]}

The scratchpad data_encoding filed should be set to 2151856 for version 1 of this protocol and
incremented by for each subsequent protocol version.

## Bored variants and their corresponding URLs

All fully qualified bored URLs should start with the bored protocol identifier ie "bored://".

### Variant 1, autonomi scratchpad addressed by the key used to create it

The key used to create and hence also decrypt the scratchpad is used as the address in hexadecimal
format as the autonomi address can be derived from this hence it will be 64 characters long eg.

## Variant 2, autonomi scratchpad addressed by the name used to derive key used to create it.

They strings used to derive the key used to create and decrypt the scratch pad. Multiple domains may
be delimited by full stops (.). The derivation start with the base key of (in hex):

    000000000000000000000000000000000000000000000000000000000020D5B0

Then a new key is derived using each domain (delimited by .) from left to right.

This mean boreds can essentially have any url name that has not already be used and there in now
specific limitation on length though be impractically long would defeat the point of having
them over random numbers...ie a human could possible remember them. The only character that cannot
be used is the full stop which are not counted as used to deliminate domains.

An empty string or any number of empty strings (ie loads of ...) are not valid addresses.

## Potential vulnerabilities

This is not strictly part of the specification, but dues to the very simple nature of the trust
mechanism used in this version of the protocol...if you have the bored address you can edit it
present a fair number of ways to disrupt the network not limited to:

### In protocol

- Noise generator: ie repeatedly spamming a bored with notices so that it can not be used for
communication.
- Homogenising wallpaper wave: Going through all links in a network and overwriting all notices
so that all network links will be lost.

### Outside protocol but within protocol data definitions

This is doing things that are not allowed within the protocol while still maintaining compliance with
the protocols data structures, eg editing a notice.

### Entirely outside protocol

- Scratchpad hijacking, using the scratchpad to store data outside of the protocol specifications.
- Scratchpad hijacking wave, using links in a network to hijack all the scratchpads in that network.

Hence future developments could include testing the effects of these on a network and developing
possible mitigations into apps using the protocol or subsequent versions of the protocol.
