"""
Quick and dirty parsing of the Raspberry Pi SDK document (https://pip-assets.raspberrypi.com/categories/609-microcontroller-boards/documents/RP-009085-KB-2-raspberry-pi-pico-c-sdk.pdf) to generate hover documentation for the pioasm instruction set
"""

import io
import re
import subprocess
import sys
from pathlib import Path

import pdfplumber

OUTFILE = Path("src/doc.rs")


def eprint(*args, **kwargs):  # noqa: ANN002, ANN003
    print(*args, file=sys.stderr, **kwargs)


def find_section_bounds_pdf(pdf: pdfplumber.pdf.PDF) -> tuple[int, int]:
    """
    Find relevant page range in the document by parsing the TOC
    """
    TOC_PAGE_LIMIT = 20

    def toc_entry_re(sec: str):
        return re.compile(rf"{re.escape(sec)}[\s\.]*(\d+)")

    sec_33_re = toc_entry_re("3.3. Using PIOASM, the PIO Assembler")
    sec_4_re = toc_entry_re("4. Signing and encrypting (RP2350 only)")

    start = None
    end = None
    for p in pdf.pages[:TOC_PAGE_LIMIT]:
        text = p.extract_text_simple()
        if start is None and (start_m := sec_33_re.search(text)) is not None:
            start = int(start_m.group(1))
        if end is None and (end_m := sec_4_re.search(text)) is not None:
            end = int(end_m.group(1))
            break

    if not start:
        raise ValueError("Could not parse start section bounds from the TOC.")
    if not end:
        raise ValueError("Could not parse end section bounds from the TOC.")
    return start, end


def to_rust_map(name: str, lst: list[tuple[str, str]], v_typ: str = "&str") -> str:
    def mk_str(e: str) -> str:
        return f'"{e.replace('"', '\\"')}"'

    out = f"pub const {name}: [(&str, {v_typ}); {len(lst)}] = ["
    for i, (k, v) in enumerate(lst):
        if i != 0:
            out += ", "
        out += f"({mk_str(k)}, {mk_str(v) if v_typ == '&str' else v})"
    return out + "];\n"


# convert note sections to markdown
NOTE_RE = re.compile(r"^( NOTE|CAUTION)\n(.*)$$", re.DOTALL | re.MULTILINE)
NOTE_TYP_RE = re.compile(r"[A-Z]+")
NOTE_CONTENT_RE = re.compile(r"^", re.MULTILINE)


def notes_to_markdown(s: str) -> str:
    def repl(match: re.Match[str]) -> str:
        typ = NOTE_TYP_RE.search(match.group(1))
        content = NOTE_CONTENT_RE.sub("> ", match.group(2))
        assert typ is not None

        return f"""
---
> **_{typ.group(0)}_**
{content}
---

"""

    return NOTE_RE.sub(repl, s)


def esc_markdown(s: str) -> str:
    return s.replace("<", "&lt;").replace(">", "&gt;")


def instr_table_name(s: str) -> str:
    return s.replace("(", "").replace(")", "").replace(" ", "_").upper()


def main() -> None:
    # consider lines with higher difference between then as new paragraph
    LINE_HEIGHT = 18.0

    sub_section_re = re.compile(r"^\d+\.\d+\.\d+\.\s(.+)$", re.MULTILINE)

    # anything between these is the instruction description
    operation_start_re = re.compile(r"^\d+\.\d+\.\d+\.\d+\.\sOperation$", re.MULTILINE)
    operation_end_re = re.compile(
        r"^\d+\.\d+\.\d+\.\d+\.\sAssembler Syntax$", re.MULTILINE
    )

    # skip pdf header/footer lines
    line_skip_re = re.compile(
        r"\d+\.\d+\. PIO Instruction Set Reference\s+\d+|Raspberry Pi Pico-series C\/C\+\+ SDK"
    )

    # remove (See Section ...)
    see_section_re = re.compile(r"\s?\([Ss]ee[ \n]+Section[ \n]+\d+\.\d+\.\d+\)")

    instructions = [
        "JMP",
        "WAIT",
        "IN",
        "OUT",
        "PUSH",
        "PULL",
        "MOV",
        "MOV (to RX)",
        "MOV (from RX)",
        "IRQ",
        "SET",
    ]

    # Assembler Syntax table for instructions
    instr_asm_syntax = {instr: [] for instr in instructions}

    # Operation sections for instruction
    instr_operation = {instr: "" for instr in instructions}

    directives = []

    with pdfplumber.open(io.BytesIO(sys.stdin.buffer.read())) as pdf:
        start_page, end_page = find_section_bounds_pdf(pdf)

        cur_sec = None
        in_operation = False
        first_line_in_operation = False

        for page in pdf.pages[start_page:end_page]:
            page_items = []

            # search for subsections
            if (m := page.search(sub_section_re)) is not None:
                for c in m:
                    page_items.append({
                        "pos": c["top"],
                        "sec": c["groups"][0],
                    })

            # operation section start/end markers
            for op_key, op_re in [
                ("operation_start", operation_start_re),
                ("operation_end", operation_end_re),
            ]:
                if (m := page.search(op_re)) is not None:
                    for c in m:
                        page_items.append({
                            "pos": c["top"],
                            op_key: None,
                        })

            # operation lines
            last_line_top = 0.0  # use distance between lines as heuristic
            for line in page.extract_text_lines():
                if line_skip_re.match(line["text"]) is not None:
                    continue

                page_items.append({
                    "pos": line["top"],
                    "bottom": line["bottom"],
                    "line": (
                        # new paragraph
                        "\n"
                        if (last_line_top != 0.0 and line["top"] - last_line_top)
                        > LINE_HEIGHT
                        else ""
                    )
                    + line["text"],
                })
                last_line_top = line["top"]

            # search for tables (for directives or Assembler Syntax)
            for table in page.find_tables():
                eprint(table.extract())
                page_items.append({
                    "pos": table.bbox[1],  # top
                    "bottom": table.bbox[3],
                    "table": table.extract(),
                })

            # sort items by vertical position on page
            page_items.sort(key=lambda e: e["pos"])

            # process page items
            for it in page_items:
                if "sec" in it:
                    # new section
                    cur_sec = it["sec"]
                    eprint()
                    eprint(f"sec: {cur_sec}")
                elif "operation_start" in it:
                    in_operation = True
                    first_line_in_operation = True
                elif "operation_end" in it:
                    in_operation = False
                elif "line" in it:
                    if not in_operation:
                        continue

                    if first_line_in_operation:
                        # skip Operation line
                        first_line_in_operation = False
                        continue

                    # line in operation section

                    eprint(f"op line: {it['line']}")

                    # convert to markdown list
                    line = it["line"].replace("•", "*").replace("◦", "  *")

                    assert cur_sec in instructions
                    instr_operation[cur_sec] += line + "\n"
                elif "table" in it:
                    table = it["table"]
                    if cur_sec in instructions:
                        # table is for instruction
                        instr_asm_syntax[cur_sec] += table
                    elif cur_sec == "Directives":
                        # table is for directive
                        directives += table

                    # for row in table:
                    #     eprint(f"r: {row} {in_operation}")
                else:
                    raise Exception(f"invalid it: {it}")

    # replace NOTE sections with markdown
    for k in instr_operation:
        instr_operation[k] = notes_to_markdown(instr_operation[k])

    # write directives
    out = to_rust_map("DIRECTIVES", directives)
    instr_doc = {instr: "" for instr in instructions}

    for k, v in instr_operation.items():
        instr_doc[k] += "\n## Operation\n" + v
        # eprint(f"des: {k}: {v}")

    for k, v in instr_asm_syntax.items():
        instr_map = {}  # use dict to dedup and preserve order
        for o in v:
            # eprint(f"op = {o}")
            if o[0] == "Bit:" or o[0] == k.split(" ")[0]:
                continue
            assert len(o) == 2, f"{len(o)}, {k}, {v}"
            instr_map[(o[0].removeprefix("`"), o[1])] = None

        # instr_doc[k] = tabulate(
        #     list(instr_map.keys()),
        #     headers=["Operand", "Description"],
        #     tablefmt="github",
        # )

        # convert to nested markdown list
        max_k = max((len(k) for k, _ in instr_map))
        instr_doc[k] += (
            "\n## Parameters \n"
            # + (instr_doc.get(k, ""))
            + "\n".join([
                f"- **{esc_markdown(k)}**{' ' * (max_k - len(k))}: {esc_markdown(v.replace('\n', ' '))}"
                for k, v in instr_map
            ])
        )

        out += to_rust_map(instr_table_name(k), [(k, v) for k, v in instr_map])

    out += to_rust_map(
        "INSTRUCTIONS",
        [(instr_table_name(k), "&" + instr_table_name(k)) for k in instr_asm_syntax],
        v_typ="&[(&str, &str)]",
    )
    out += to_rust_map(
        "INSTRUCTION_DOC",
        [(instr_table_name(k), v) for k, v in instr_doc.items()],
    )

    # write and format output
    OUTFILE.write_text(
        "// auto-generated, do not edit\n\n" + see_section_re.sub("", out)
    )
    subprocess.run(["rustfmt", OUTFILE])


if __name__ == "__main__":
    main()
