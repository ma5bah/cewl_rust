# CeWL - Custom Word List generator (Rust port)

> **Stolen from [digininja/CeWL](https://github.com/digininja/CeWL) by Robin Wood (robin@digi.ninja) — and rewritten in Rust.**  
> The original Ruby source was stolen borrowed as the direct reference for this port.  
> All credit for the original concept, design, and logic belongs to Robin Wood. We just made it faster. 🦀

Spiders a given URL to a specified depth and returns a list of words for use with password crackers such as John the Ripper.

By default CeWL sticks to the target site, goes to a depth of 2 links, and outputs all words of 3 characters or more. This Rust port adds headless Chromium rendering by default (falls back to static HTTP) and a human-in-loop mode for bypassing CAPTCHAs or logins.

CeWL also includes **FAB** (Files Already Bagged), a companion tool that extracts author/creator metadata from already-downloaded files.

Original Ruby project: <https://github.com/digininja/CeWL>  
This Rust port: <https://github.com/ma5bah/cewl_rust>  
Homepage: <https://digi.ninja/projects/cewl.php>

## Installation

### From source

Requires Rust 1.70+ (`rustup.rs`) and optionally `exiftool` for legacy Office metadata.

```bash
git clone https://github.com/digininja/CeWL.git
cd CeWL
cargo build --release
# binaries: target/release/cewl  target/release/fab
```

Make available system-wide (optional):

```bash
sudo cp target/release/cewl target/release/fab /usr/local/bin/
```

Verify:

```bash
cewl --version
fab --version
```

### Docker

```bash
# Build
docker build -f Dockerfile.rust -t cewl .

# Run
docker run --rm cewl [OPTIONS] <URL>
docker run --rm -v "${PWD}:/host" cewl --write /host/words.txt [OPTIONS] <URL>
```

The Docker image includes Chromium and exiftool.

## Usage

```
cewl [OPTIONS] <URL>

OPTIONS:
    -h, --help                    Show help
    -k, --keep                    Keep the downloaded metadata temp file
    -d, --depth <x>               Depth to spider to (default: 2)
    -m, --min-word-length <x>     Minimum word length (default: 3)
    -x, --max-word-length <x>     Maximum word length (default: unset)
    -o, --offsite                 Let the spider visit other sites
        --exclude <file>          File containing paths to exclude
        --allowed <regex>         Regex that path must match to be followed
    -w, --write <file>            Write word output to file
    -u, --ua <agent>              User-Agent string
    -n, --no-words                Don't output the wordlist
    -g, --groups <x>              Return groups of x words as well
        --lowercase               Lowercase all parsed words
        --with-numbers            Accept words with numbers
        --convert-umlauts         Convert ISO-8859-1 umlauts (ä→ae, ö→oe, ü→ue, ß→ss)
    -a, --meta                    Include metadata from documents
        --meta-file <file>        Output file for meta data
    -e, --email                   Include email addresses
        --email-file <file>       Output file for email addresses
        --meta-temp-dir <dir>     Temp dir used for metadata downloads (default: /tmp)
    -c, --count                   Show the count for each word found
    -v, --verbose                 Verbose
        --debug                   Extra debug information

    Authentication
        --auth-type               basic or digest
        --auth-user               Authentication username
        --auth-pass               Authentication password

    Proxy
        --proxy-host              Proxy host
        --proxy-port              Proxy port (default: 8080)
        --proxy-username          Proxy username
        --proxy-password          Proxy password

    Headers
    -H, --header <name:value>     Extra header (repeatable)

    URL Structure Capture
        --capture-paths           Add URL path components to the wordlist
        --capture-subdomains      Add subdomain components to the wordlist
        --capture-domain          Add the registrable domain to the wordlist
        --capture-url-structure   All of the above

    Browser (Chromium)
        --no-render               Disable Chromium; use static HTTP only
        --render-wait             Page event to wait for: load | domcontentloaded | networkidle
                                  (default: domcontentloaded)
        --render-timeout <secs>   Navigation timeout in seconds (default: 20)
        --browser-path <path>     Path to Chrome/Chromium binary
        --browser-user-data-dir   Chrome user-data-dir for persistent profile
                                  (implies --headed; useful for login/CAPTCHA sessions)
        --browser-profile-dir     Profile directory inside user-data-dir (e.g. "Profile 1")
        --headed                  Show the browser window
        --human-in-loop           Pause before each fetch so a human can solve CAPTCHAs
        --human-timeout <secs>    Seconds to wait for human input (default: 300)
        --concurrency <n>         Max concurrent page fetches (default: 4)
        --no-fallback             Do not fall back to static HTTP on browser failure
        --insecure                Disable TLS certificate verification
        --no-sandbox              Pass --no-sandbox to Chromium (required in containers)
        --max-pages <n>           Stop after n pages (0 = unlimited)

    <URL>: The site to spider
```

> **Note:** Ruby-style underscore flags (`--min_word_length`, `--auth_user`, `--proxy_host`, etc.) are accepted as aliases.

### FAB — Files Already Bagged

Extract author/creator metadata from local files:

```bash
fab [files...]
fab report.pdf notes.docx presentation.pptx
```

Supports PDF (native byte-level extraction), DOCX/XLSX/PPTX (OOXML ZIP), and legacy Office formats via `exiftool`.

### Examples

```bash
# Basic crawl, depth 2, static HTTP
cewl --no-render https://example.com

# Depth 3, collect emails and metadata, write to files
cewl -d 3 -e --email-file emails.txt -a --meta-file meta.txt -w words.txt https://example.com

# Browser crawl waiting for full page load
cewl --render-wait load https://example.com

# Human-in-loop: pause on each page for CAPTCHA solving
cewl --browser-user-data-dir ~/.config/google-chrome --human-in-loop https://example.com

# Inside Docker (no-sandbox required)
docker run --rm cewl --no-sandbox -d 2 https://example.com
```

## Licence

This project is released under the Creative Commons Attribution-Share Alike 2.0 UK: England & Wales.

<http://creativecommons.org/licenses/by-sa/2.0/uk/>

Alternatively, you may use GPL-3+ instead of the original licence.

<http://opensource.org/licenses/GPL-3.0>
