# Usage examples

`commands.txt` is a short script of REPL input: three `insert`s, a
`select`, and the `.btree`/`.constants` meta-commands. Pipe it into the
built binary against a database file under `data/` (which is `.gitignore`d
apart from its `.gitkeep`, so generated `.db` files never get committed):

```sh
cargo run -- data/example.db < examples/commands.txt
```

Expected output:

```
db > Executed.
db > Executed.
db > Executed.
db > (1, alice, alice@example.com)
(2, bob, bob@example.com)
(3, carol, carol@example.com)
Executed.
db > Tree:
- leaf (size 3)
  - 1
  - 2
  - 3
db > Constants:
ROW_SIZE: 293
COMMON_NODE_HEADER_SIZE: 6
LEAF_NODE_HEADER_SIZE: 14
LEAF_NODE_CELL_SIZE: 297
LEAF_NODE_SPACE_FOR_CELLS: 4082
LEAF_NODE_MAX_CELLS: 13
db >
```

Because the database file persists on disk, rerunning the same command
with just `select\n.exit\n` piped in (or typed interactively) shows the
same three rows without re-inserting them:

```sh
printf 'select\n.exit\n' | cargo run -- data/example.db
```

See `../docs/file-format.md` for the on-disk byte layout these commands
produce, including a note on the current single-leaf-page row limit.
