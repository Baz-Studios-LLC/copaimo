# What this is, and how to put it back

Claude's memory for this machine, copied into the repository on 2026-09-26 because
the laptop it lived on was about to be reset. It is the one part of this project that
was not under version control and could not be rebuilt: seventy-one facts learned
across months of sessions — which decisions were made and why, which approaches were
tried and rejected, which mistakes were made twice.

It normally lives **outside** any repository, at:

    ~/.claude/projects/<project-path-slug>/memory/

where the slug is the working directory with separators replaced by dashes — on the
machine this came from, `C--Users-jsull-Desktop`, because sessions were started from
the Desktop rather than from inside this repository.

## Restoring it

Copy the `*.md` files (NOT this README) into that folder on the new machine:

```bash
mkdir -p ~/.claude/projects/C--Users-jsull-Desktop/memory
cp .claude/memory/*.md ~/.claude/projects/C--Users-jsull-Desktop/memory/
rm ~/.claude/projects/C--Users-jsull-Desktop/memory/README.md
```

`MEMORY.md` is the index that gets loaded into context each session; the rest are one
fact per file and are read on demand. If sessions are started from a different
directory on the new machine the slug changes, and the folder name has to change with
it or nothing will be found.

## A caveat worth keeping

These describe the project as it was when each was written. Several name specific
files, functions and constants; those move. A memory is a pointer worth checking, not
a fact to act on unread — which is what the memories themselves say about themselves.

## The rest of this folder is not Claude's

`.claude/worktrees/` is scratch for parallel agents and is ignored on purpose. Nothing
else in here is needed to build or run the game.
