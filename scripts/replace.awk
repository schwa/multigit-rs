# Awk script to replace a code block (at INDEX) in a markdown file with the output of a command (REPLACEMENT).
#
# awk -f replace.awk -v INDEX=2 -v "REPLACEMENT=cargo run -- --help 2> /dev/null" README.md | sponge README.md

match($0, "```[a-z]+$") {
    count += 1;
    if (count == INDEX) {
        skip = 1;
    }
}
skip == 0 { print }
match($0, "```$") {
    skip = 0;
    if (count == INDEX) {
        print("```sh")
        system(REPLACEMENT)
        print("```")
    }
}
