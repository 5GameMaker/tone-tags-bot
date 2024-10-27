# Tone Tags bot

A bot that gives info on tone tags as well as hopefully helpful tips on how to better
use the system.

## Installation

Go to <https://discord.com/oauth2/authorize?client_id=614705195108007957> and select whatever
server you want or just install it as a user app.

## Stuff

- [Privacy Policy](PRIVACY.md)
- [Terms of Service](TERMS.md)

## Contributing tone tags

All information about tone tags is located in `standards/`.

Tone tag standards must have a structure of:
```md
# Standard name
Standard description

## /tag /s
Tag description
> :warning:
> This is a warning telling you that use of this tag
> should be avoided as all costs!
>
> Consider not using this tone tag

> :information_source:
> Some information 
```

When writing or contributing to a standard:
- There must be no dots at the end
- If possible, referencinig the message or message contents should be avoided
- Prefer clarity over nice wording. If fancy words are necessary, consider adding
  a [link](https://en.wikipedia.org/wiki/Hyperlink) to their defition
- No spelling mistakes

If you found a way to improve on any of the above, consider sending a PR!

## Contributing code

If the fix is small, simply clone the repo and submit a PR. For anything larger
consider opening a feature request and transforming it later into a tracking
issue if approved.

Whatever code is added should be compatible with running it via [`proxychains`](https://github.com/haad/proxychains).
