# We are bored protocol

Version: 3

## Abstract

The we are [bored](bored.acronyms) protocol is a minimalist ephemeral social interaction network.
It is implemented using decentralized computer-to-computer secure networking via the [x0x](skill.md) protocol and based on the [SMILE](SMILE_Philosophy.md) philosophy, though it falls short of some of those principles.

Its inspiration is real-world physical notice boards. It is made up of "boreds", which are two-dimensional volumes containing notices. These notices contain text which may include hyperlinks that can point to other boreds in the network. These may be viewed by anyone who has the bored's address, and anyone with access may also add notices in a decentralized manner.

---

## Definitions

### Coordinates

All coordinates begin at the top left (0, 0).

### Boreds

- A bored is a named two-dimensional volume containing a collection of notices positioned on the board. All notices must sit entirely within the board and must be at least partially visible.
- As new notices are added, they are placed on top of any existing notices occupying the same space.
- It is possible to view the entirety of a notice, even if parts of it are occluded by subsequent notices.
- When a notice becomes entirely occluded by subsequent notices, it no longer exists (it is pruned).
- The x and y dimensions of the bored must be whole numbers between 0 and 65535 (i.e., unsigned 16-bit integers), representing character widths and heights.
- The bored data is cached locally as a JSON file under `~/.local/share/we-are-bored/cache/<topic>.json` to enable offline loading and history persistence.
- To maintain size constraints and performance, when a board is modified, any notices that are entirely occluded are immediately pruned using a deterministic visibility layout calculation (`WhatsOnTheBored`).

### Character

In the context of this specification, a character is a [Unicode scalar value](https://www.unicode.org/glossary/#unicode_scalar_value).

### Notices

- A notice is a rectangle that has a capacity to hold text of its volume minus 1 character at the edges, which is used as a border. Hence, a 3x3 notice only has a 1-character text capacity.
- A notice's capacity does not include the non-visible elements of a hyperlink (markdown URL syntax) at render time.
- A notice may contain as many hyperlinks as its character capacity allows.
- A notice cannot be edited once placed on the bored.
- To support deterministic deduplication and synchronization across decentralized peers, each notice is assigned a globally unique `notice_id` when drafted.

### Hyperlinks

- Hyperlinks are represented in the text of a notice using standard markdown notation `[Text](URL)`.
- The brackets and URL of the hyperlink are not rendered and are not included in calculating a notice's text capacity.
- To prevent abuse, reasonable limits are imposed on the length of the URL text (up to 2048 characters).
- Hyperlink URLs can represent:
  - **Bored URLs**: Starting with `bored://`, pointing to other boards.
  - **App URLs**: Starting with `app://`, used for client application navigation (e.g., `app://home`, `app://about`).
  - **Clearnet URLs**: Starting with `http://` or `https://`.
- **Legacy autonomy and `ant://` addresses are no longer supported.**

---

## Data Formats

### Notice Structure

A notice contains the following fields:

- `notice_id`: A globally unique string identifier. It is generated deterministically as `notice:<timestamp_ms>:<agent_id_prefix>` (using the first 8 characters of the local `x0x` agent ID) to prevent collisions.
- `top_left`: The coordinate of the top-left corner of the notice with respect to the board.
- `dimensions`: The dimensions (width and height) of the notice.
- `content`: The raw text content of the notice (including markdown hyperlinks).

### Bored Structure

A bored contains the following fields:

- `protocol_version`: The protocol version integer (set to `3`).
- `name`: The name of the bored.
- `dimensions`: The coordinates of the bottom-right bounds of the bored.
- `notices`: A collection of active, visible notices.

### JSON Representation Example

The board data is represented in JSON format. Below is an example matching Version 3:

```json
{
  "protocol_version": 3,
  "name": "The genesis bored",
  "dimensions": {
    "x": 120,
    "y": 40
  },
  "notices": [
    {
      "notice_id": "notice:1779796800000:abc123ef",
      "top_left": {
        "x": 6,
        "y": 2
      },
      "dimensions": {
        "x": 30,
        "y": 8
      },
      "content": "Hello is this bored for fans\nof genesis?"
    },
    {
      "notice_id": "notice:1779796850000:abc123ef",
      "top_left": {
        "x": 17,
        "y": 7
      },
      "dimensions": {
        "x": 30,
        "y": 10
      },
      "content": "No it's just the first bored\nto ever exist."
    },
    {
      "notice_id": "notice:1779796900000:xyz987ab",
      "top_left": {
        "x": 71,
        "y": 25
      },
      "dimensions": {
        "x": 30,
        "y": 10
      },
      "content": "This is a [link to another board](bored://welcome)"
    }
  ]
}
```

---

## Decentralized Gossip Synchronization via x0x

Version 3 of the protocol implements synchronization through a secure, peer-to-peer gossip pub/sub model utilizing a local `x0xd` daemon.

### Local Daemon Integration

The client communicates with the local `x0xd` daemon via its REST and Server-Sent Events (SSE) API:
- **Credentials Discovery**: Reads the daemon api port and token from `~/.local/share/x0x/api.port` and `~/.local/share/x0x/api-token`.
- **SSE Stream**: Establishes a persistent background listener at `/events` to receive gossip messages from the network.
- **REST Actions**: Publishes new events via the `/publish` endpoint and manages subscriptions via the `/subscribe` endpoint.
- **Connection Checks**: Automatically detects connection failures, checking if the daemon is starting or installing, and offering guides to start or install the service.

### Gossip Messages (`GossipMsg`)

All synchronization data transmitted over the gossip network is encapsulated in a JSON structure and base64-encoded as a payload on a specific gossip topic (prefixed as `bored.<board_topic>`).

The gossip message types (discriminated by the `type` tag) are:

1. **`meta`**:
   Used to broadcast the board's name and coordinates when created.
   ```json
   {
     "type": "meta",
     "name": "Board Name",
     "dimensions": { "x": 120, "y": 40 }
   }
   ```

2. **`notice`**:
   Broadcasts a new notice placed on the board.
   ```json
   {
     "type": "notice",
     "notice": {
       "notice_id": "notice:1779796800000:abc123ef",
       "top_left": { "x": 5, "y": 5 },
       "dimensions": { "x": 10, "y": 5 },
       "content": "Gossip update!"
     }
   }
   ```

3. **`sync-request`**:
   Broadcasted when joining or refreshing a board to request the current state of notices from any online peer.
   ```json
   {
     "type": "sync-request"
   }
   ```

4. **`sync-response`**:
   Sent by online peers in response to a `sync-request`, carrying the board name, dimensions, and all currently active notices to synchronize a joining client.
   ```json
   {
     "type": "sync-response",
     "name": "Sync Board",
     "dimensions": { "x": 120, "y": 40 },
     "notices": [ ... ]
   }
   ```

---

## Bored URL Variants and Topics

All fully qualified bored URLs start with the `bored://` protocol identifier.

### Variant 1: Topic-Based Address

A random, unique address represented as:
```
bored://bored.<uuid>
```
where `<uuid>` is a standard UUIDv4 identifier. The corresponding gossip pub/sub topic subscribed to and published on is exactly `bored.<uuid>`.

### Variant 2: Human-Readable Named Address

A friendly name representation:
```
bored://<name>
```
where `<name>` is a human-readable string (e.g., `bored://welcome`). This maps directly to the gossip pub/sub topic `bored.<name>`.

---

## Potential Vulnerabilities

Due to the highly minimalist, decentralized nature of the trust model used in this version of the protocol, any user with a board's address can write to it. This introduces several potential ways to disrupt the network:

### In-Protocol Vulnerabilities

- **Noise Generator**: Repeatedly spamming a board with new notices to occlude existing conversations and make the board unusable.
- **Homogenizing Wallpaper Wave**: Iterating through links on boards to automatically place giant blank or uniform notices that cover and destroy all existing content and hyperlinks.

### Outside Protocol but within Data Definitions

- **ID Collisions / Hijacking**: Deliberately generating duplicate `notice_id` values matching existing notices to prevent new notices from being accepted by peers, or simulating messages with forged timestamps.

### Entirely Outside Protocol

- **Gossip Topic Flooding**: Flooding the subscribed `x0x` gossip topics with non-protocol payloads, oversized JSON payloads, or junk binary data to exhaust system resources or disrupt connection streams.
- **Network Poisoning**: Spreading inconsistent `sync-response` payloads with conflicting notice sets to cause state desynchronization between different online nodes.
