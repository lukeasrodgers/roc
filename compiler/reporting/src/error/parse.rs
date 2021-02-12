use roc_parse::parser::{Col, ParseProblem, Row, SyntaxError};
use roc_region::all::Region;
use std::path::PathBuf;

use crate::report::{Report, RocDocAllocator, RocDocBuilder};
use ven_pretty::DocAllocator;

pub fn parse_problem<'a>(
    alloc: &'a RocDocAllocator<'a>,
    filename: PathBuf,
    _starting_line: u32,
    parse_problem: ParseProblem<SyntaxError<'a>>,
) -> Report<'a> {
    to_syntax_report(
        alloc,
        filename,
        &parse_problem.problem,
        parse_problem.line,
        parse_problem.column,
    )
}

fn note_for_record_type_indent<'a>(alloc: &'a RocDocAllocator<'a>) -> RocDocBuilder<'a> {
    alloc.note("I may be confused by indentation")
}

fn note_for_tag_union_type_indent<'a>(alloc: &'a RocDocAllocator<'a>) -> RocDocBuilder<'a> {
    alloc.note("I may be confused by indentation")
}

fn hint_for_tag_name<'a>(alloc: &'a RocDocAllocator<'a>) -> RocDocBuilder<'a> {
    alloc.concat(vec![
        alloc.hint("Tag names "),
        alloc.reflow("start with an uppercase letter, like "),
        alloc.parser_suggestion("Err"),
        alloc.text(" or "),
        alloc.parser_suggestion("Green"),
        alloc.text("."),
    ])
}

fn hint_for_private_tag_name<'a>(alloc: &'a RocDocAllocator<'a>) -> RocDocBuilder<'a> {
    alloc.concat(vec![
        alloc.hint("Private tag names "),
        alloc.reflow("start with a `@` symbol followed by an uppercase letter, like "),
        alloc.parser_suggestion("@UID"),
        alloc.text(" or "),
        alloc.parser_suggestion("@SecretKey"),
        alloc.text("."),
    ])
}

fn to_syntax_report<'a>(
    alloc: &'a RocDocAllocator<'a>,
    filename: PathBuf,
    parse_problem: &roc_parse::parser::SyntaxError<'a>,
    start_row: Row,
    start_col: Col,
) -> Report<'a> {
    use SyntaxError::*;

    let report = |doc| Report {
        filename: filename.clone(),
        doc,
        title: "PARSE PROBLEM".to_string(),
    };

    let region = Region {
        start_line: start_row,
        end_line: start_row,
        start_col,
        end_col: start_col + 1,
    };

    match parse_problem {
        SyntaxError::ConditionFailed => {
            let doc = alloc.stack(vec![
                alloc.reflow("A condition failed:"),
                alloc.region(region),
            ]);

            Report {
                filename,
                doc,
                title: "PARSE PROBLEM".to_string(),
            }
        }
        SyntaxError::ArgumentsBeforeEquals(region) => {
            let doc = alloc.stack(vec![
                alloc.reflow("Unexpected tokens in front of the `=` symbol:"),
                alloc.region(*region),
            ]);

            Report {
                filename,
                doc,
                title: "PARSE PROBLEM".to_string(),
            }
        }
        Unexpected(mut region) => {
            if region.start_col == region.end_col {
                region.end_col += 1;
            }

            let doc = alloc.stack(vec![
                alloc.concat(vec![
                    alloc.reflow("Unexpected token "),
                    // context(alloc, &parse_problem.context_stack, "here"),
                    alloc.text(":"),
                ]),
                alloc.region(region),
            ]);

            report(doc)
        }
        SyntaxError::Eof(region) => {
            let doc = alloc.stack(vec![alloc.reflow("End of Field"), alloc.region(*region)]);

            Report {
                filename,
                doc,
                title: "PARSE PROBLEM".to_string(),
            }
        }
        SyntaxError::OutdentedTooFar => {
            let doc = alloc.stack(vec![alloc.reflow("OutdentedTooFar")]);

            Report {
                filename,
                doc,
                title: "PARSE PROBLEM".to_string(),
            }
        }
        Type(typ) => to_type_report(alloc, filename, &typ, 0, 0),
        _ => todo!("unhandled parse error: {:?}", parse_problem),
    }
}

fn to_type_report<'a>(
    alloc: &'a RocDocAllocator<'a>,
    filename: PathBuf,
    parse_problem: &roc_parse::parser::Type<'a>,
    start_row: Row,
    start_col: Col,
) -> Report<'a> {
    use roc_parse::parser::Type;

    match parse_problem {
        Type::TRecord(record, row, col) => to_trecord_report(alloc, filename, &record, *row, *col),
        Type::TTagUnion(tag_union, row, col) => {
            to_ttag_union_report(alloc, filename, &tag_union, *row, *col)
        }
        Type::TInParens(tinparens, row, col) => {
            to_tinparens_report(alloc, filename, &tinparens, *row, *col)
        }
        Type::TApply(tapply, row, col) => to_tapply_report(alloc, filename, &tapply, *row, *col),

        Type::TFunctionArgument(row, col) => match what_is_next(alloc.src_lines, *row, *col) {
            Next::Other(Some(',')) => {
                let surroundings = Region::from_rows_cols(start_row, start_col, *row, *col);
                let region = Region::from_row_col(*row, *col);

                let doc = alloc.stack(vec![
                    alloc.reflow(r"I just started parsing a function argument type, but I encounterd two commas in a row:"),
                    alloc.region_with_subregion(surroundings, region),
                    alloc.concat(vec![alloc.reflow("Try removing one of them.")]),
                ]);

                Report {
                    filename,
                    doc,
                    title: "DOUBLE COMMA".to_string(),
                }
            }
            _ => todo!(),
        },

        Type::TStart(row, col) => {
            let surroundings = Region::from_rows_cols(start_row, start_col, *row, *col);
            let region = Region::from_row_col(*row, *col);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I just started parsing a type, but I got stuck here:"),
                alloc.region_with_subregion(surroundings, region),
                alloc.note("I may be confused by indentation"),
            ]);

            Report {
                filename,
                doc,
                title: "UNFINISHED TYPE".to_string(),
            }
        }

        Type::TIndentStart(row, col) => {
            let surroundings = Region::from_rows_cols(start_row, start_col, *row, *col);
            let region = Region::from_row_col(*row, *col);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I just started parsing a type, but I got stuck here:"),
                alloc.region_with_subregion(surroundings, region),
                alloc.note("I may be confused by indentation"),
            ]);

            Report {
                filename,
                doc,
                title: "UNFINISHED TYPE".to_string(),
            }
        }

        Type::TAsIndentStart(row, col) => {
            let surroundings = Region::from_rows_cols(start_row, start_col, *row, *col);
            let region = Region::from_row_col(*row, *col);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I just started parsing an inline type alias, but I got stuck here:"),
                alloc.region_with_subregion(surroundings, region),
                alloc.note("I may be confused by indentation"),
            ]);

            Report {
                filename,
                doc,
                title: "UNFINISHED INLINE ALIAS".to_string(),
            }
        }

        _ => todo!("unhandled type parse error: {:?}", &parse_problem),
    }
}

fn to_trecord_report<'a>(
    alloc: &'a RocDocAllocator<'a>,
    filename: PathBuf,
    parse_problem: &roc_parse::parser::TRecord<'a>,
    start_row: Row,
    start_col: Col,
) -> Report<'a> {
    use roc_parse::parser::TRecord;

    match *parse_problem {
        TRecord::Open(row, col) => match what_is_next(alloc.src_lines, row, col) {
            Next::Keyword(keyword) => {
                let surroundings = Region::from_rows_cols(start_row, start_col, row, col);
                let region = to_keyword_region(row, col, keyword);

                let doc = alloc.stack(vec![
                    alloc.reflow(r"I just started parsing a record type, but I got stuck on this field name:"),
                    alloc.region_with_subregion(surroundings, region),
                    alloc.concat(vec![
                        alloc.reflow(r"Looks like you are trying to use "),
                        alloc.keyword(keyword),
                        alloc.reflow(" as a field name, but that is a reserved word. Try using a different name!"),
                    ]),
                ]);

                Report {
                    filename,
                    doc,
                    title: "UNFINISHED RECORD TYPE".to_string(),
                }
            }
            _ => {
                let surroundings = Region::from_rows_cols(start_row, start_col, row, col);
                let region = Region::from_row_col(row, col);

                let doc = alloc.stack(vec![
                    alloc.reflow(r"I just started parsing a record type, but I got stuck here:"),
                    alloc.region_with_subregion(surroundings, region),
                    alloc.concat(vec![
                        alloc.reflow(r"Record types look like "),
                        alloc.parser_suggestion("{ name : String, age : Int },"),
                        alloc.reflow(" so I was expecting to see a field name next."),
                    ]),
                ]);

                Report {
                    filename,
                    doc,
                    title: "UNFINISHED RECORD TYPE".to_string(),
                }
            }
        },

        TRecord::End(row, col) => {
            let surroundings = Region::from_rows_cols(start_row, start_col, row, col);
            let region = Region::from_row_col(row, col);

            match what_is_next(alloc.src_lines, row, col) {
                Next::Other(Some(c)) if c.is_alphabetic() => {
                    let doc = alloc.stack(vec![
                        alloc.reflow(r"I am partway through parsing a record type, but I got stuck here:"),
                        alloc.region_with_subregion(surroundings, region),
                        alloc.concat(vec![
                            alloc.reflow(
                                r"I was expecting to see a colon, question mark, comma or closing curly brace.",
                            ),
                        ]),
                    ]);

                    Report {
                        filename,
                        doc,
                        title: "UNFINISHED RECORD TYPE".to_string(),
                    }
                }
                _ => {
                    let doc = alloc.stack(vec![
                alloc.reflow("I am partway through parsing a record type, but I got stuck here:"),
                alloc.region_with_subregion(surroundings, region),
                alloc.concat(vec![
                    alloc.reflow(
                        r"I was expecting to see a closing curly brace before this, so try adding a ",
                    ),
                    alloc.parser_suggestion("}"),
                    alloc.reflow(" and see if that helps?"),
                ]),
            ]);

                    Report {
                        filename,
                        doc,
                        title: "UNFINISHED RECORD TYPE".to_string(),
                    }
                }
            }
        }

        TRecord::Field(row, col) => match what_is_next(alloc.src_lines, row, col) {
            Next::Keyword(keyword) => {
                let surroundings = Region::from_rows_cols(start_row, start_col, row, col);
                let region = to_keyword_region(row, col, keyword);

                let doc = alloc.stack(vec![
                    alloc.reflow(r"I just started parsing a record type, but I got stuck on this field name:"),
                    alloc.region_with_subregion(surroundings, region),
                    alloc.concat(vec![
                        alloc.reflow(r"Looks like you are trying to use "),
                        alloc.keyword(keyword),
                        alloc.reflow(" as a field name, but that is a reserved word. Try using a different name!"),
                    ]),
                ]);

                Report {
                    filename,
                    doc,
                    title: "UNFINISHED RECORD TYPE".to_string(),
                }
            }
            Next::Other(Some(',')) => todo!(),
            Next::Other(Some('}')) => unreachable!("or is it?"),
            _ => {
                let surroundings = Region::from_rows_cols(start_row, start_col, row, col);
                let region = Region::from_row_col(row, col);

                let doc = alloc.stack(vec![
                    alloc.reflow(r"I am partway through parsing a record type, but I got stuck here:"),
                    alloc.region_with_subregion(surroundings, region),
                    alloc.concat(vec![
                        alloc.reflow(r"I was expecting to see another record field defined next, so I am looking for a name like "),
                        alloc.parser_suggestion("userName"),
                        alloc.reflow(" or "),
                        alloc.parser_suggestion("plantHight"),
                        alloc.reflow("."),
                    ]),
                ]);

                Report {
                    filename,
                    doc,
                    title: "PROBLEM IN RECORD TYPE".to_string(),
                }
            }
        },

        TRecord::Colon(_, _) => {
            unreachable!("because `{ foo }` is a valid field; the colon is not required")
        }
        TRecord::Optional(_, _) => {
            unreachable!("because `{ foo }` is a valid field; the question mark is not required")
        }

        TRecord::Type(tipe, row, col) => to_type_report(alloc, filename, tipe, row, col),

        TRecord::IndentOpen(row, col) => {
            let surroundings = Region::from_rows_cols(start_row, start_col, row, col);
            let region = Region::from_row_col(row, col);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I just started parsing a record type, but I got stuck here:"),
                alloc.region_with_subregion(surroundings, region),
                alloc.concat(vec![
                    alloc.reflow(r"Record types look like "),
                    alloc.parser_suggestion("{ name : String, age : Int },"),
                    alloc.reflow(" so I was expecting to see a field name next."),
                ]),
                note_for_record_type_indent(alloc),
            ]);

            Report {
                filename,
                doc,
                title: "UNFINISHED RECORD TYPE".to_string(),
            }
        }

        TRecord::IndentEnd(row, col) => {
            match next_line_starts_with_close_curly(alloc.src_lines, row.saturating_sub(1)) {
                Some((curly_row, curly_col)) => {
                    let surroundings =
                        Region::from_rows_cols(start_row, start_col, curly_row, curly_col);
                    let region = Region::from_row_col(curly_row, curly_col);

                    let doc = alloc.stack(vec![
                        alloc.reflow(
                            "I am partway through parsing a record type, but I got stuck here:",
                        ),
                        alloc.region_with_subregion(surroundings, region),
                        alloc.concat(vec![
                            alloc.reflow("I need this curly brace to be indented more. Try adding more spaces before it!"),
                        ]),
                    ]);

                    Report {
                        filename,
                        doc,
                        title: "NEED MORE INDENTATION".to_string(),
                    }
                }
                None => {
                    let surroundings = Region::from_rows_cols(start_row, start_col, row, col);
                    let region = Region::from_row_col(row, col);

                    let doc = alloc.stack(vec![
                        alloc.reflow(
                            r"I am partway through parsing a record type, but I got stuck here:",
                        ),
                        alloc.region_with_subregion(surroundings, region),
                        alloc.concat(vec![
                            alloc.reflow("I was expecting to see a closing curly "),
                            alloc.reflow("brace before this, so try adding a "),
                            alloc.parser_suggestion("}"),
                            alloc.reflow(" and see if that helps?"),
                        ]),
                        note_for_record_type_indent(alloc),
                    ]);

                    Report {
                        filename,
                        doc,
                        title: "UNFINISHED RECORD TYPE".to_string(),
                    }
                }
            }
        }

        TRecord::IndentColon(_, _) => {
            unreachable!("because `{ foo }` is a valid field; the colon is not required")
        }

        TRecord::IndentOptional(_, _) => {
            unreachable!("because `{ foo }` is a valid field; the question mark is not required")
        }

        TRecord::Space(error, row, col) => to_space_report(alloc, filename, &error, row, col),
    }
}

fn to_ttag_union_report<'a>(
    alloc: &'a RocDocAllocator<'a>,
    filename: PathBuf,
    parse_problem: &roc_parse::parser::TTagUnion<'a>,
    start_row: Row,
    start_col: Col,
) -> Report<'a> {
    use roc_parse::parser::TTagUnion;

    match *parse_problem {
        TTagUnion::Open(row, col) => match what_is_next(alloc.src_lines, row, col) {
            Next::Keyword(keyword) => {
                let surroundings = Region::from_rows_cols(start_row, start_col, row, col);
                let region = to_keyword_region(row, col, keyword);

                let doc = alloc.stack(vec![
                    alloc.reflow(r"I just started parsing a tag union, but I got stuck on this field name:"),
                    alloc.region_with_subregion(surroundings, region),
                    alloc.concat(vec![
                        alloc.reflow(r"Looks like you are trying to use "),
                        alloc.keyword(keyword),
                        alloc.reflow(" as a tag name, but that is a reserved word. Tag names must start with a uppercase letter."),
                    ]),
                ]);

                Report {
                    filename,
                    doc,
                    title: "UNFINISHED TAG UNION TYPE".to_string(),
                }
            }
            Next::Other(Some(c)) if c.is_alphabetic() => {
                debug_assert!(c.is_lowercase());

                let surroundings = Region::from_rows_cols(start_row, start_col, row, col);
                let region = Region::from_row_col(row, col);

                let doc = alloc.stack(vec![
                    alloc.reflow(
                        r"I am partway through parsing a tag union type, but I got stuck here:",
                    ),
                    alloc.region_with_subregion(surroundings, region),
                    alloc.reflow(r"I was expecting to see a tag name."),
                    hint_for_tag_name(alloc),
                ]);

                Report {
                    filename,
                    doc,
                    title: "WEIRD TAG NAME".to_string(),
                }
            }
            _ => {
                let surroundings = Region::from_rows_cols(start_row, start_col, row, col);
                let region = Region::from_row_col(row, col);

                let doc = alloc.stack(vec![
                    alloc.reflow(r"I just started parsing a tag union type, but I got stuck here:"),
                    alloc.region_with_subregion(surroundings, region),
                    alloc.concat(vec![
                        alloc.reflow(r"Tag unions look like "),
                        alloc.parser_suggestion("[ Many I64, None ],"),
                        alloc.reflow(" so I was expecting to see a tag name next."),
                    ]),
                ]);

                Report {
                    filename,
                    doc,
                    title: "UNFINISHED TAG UNION TYPE".to_string(),
                }
            }
        },

        TTagUnion::End(row, col) => {
            let surroundings = Region::from_rows_cols(start_row, start_col, row, col);
            let region = Region::from_row_col(row, col);

            match what_is_next(alloc.src_lines, row, col) {
                Next::Other(Some(c)) if c.is_alphabetic() => {
                    debug_assert!(c.is_lowercase());

                    let doc = alloc.stack(vec![
                        alloc.reflow(
                            r"I am partway through parsing a tag union type, but I got stuck here:",
                        ),
                        alloc.region_with_subregion(surroundings, region),
                        alloc.reflow(r"I was expecting to see a tag name."),
                        hint_for_tag_name(alloc),
                    ]);

                    Report {
                        filename,
                        doc,
                        title: "WEIRD TAG NAME".to_string(),
                    }
                }
                Next::Other(Some('@')) => {
                    let doc = alloc.stack(vec![
                        alloc.reflow(
                            r"I am partway through parsing a tag union type, but I got stuck here:",
                        ),
                        alloc.region_with_subregion(surroundings, region),
                        alloc.reflow(r"I was expecting to see a private tag name."),
                        hint_for_private_tag_name(alloc),
                    ]);

                    Report {
                        filename,
                        doc,
                        title: "WEIRD TAG NAME".to_string(),
                    }
                }
                _ => {
                    let doc = alloc.stack(vec![
                        alloc.reflow(r"I am partway through parsing a tag union type, but I got stuck here:"),
                        alloc.region_with_subregion(surroundings, region),
                        alloc.concat(vec![
                                alloc.reflow(
                                    r"I was expecting to see a closing square bracket before this, so try adding a ",
                                ),
                                alloc.parser_suggestion("]"),
                                alloc.reflow(" and see if that helps?"),
                            ]),
                        ]);

                    Report {
                        filename,
                        doc,
                        title: "UNFINISHED TAG UNION TYPE".to_string(),
                    }
                }
            }
        }

        TTagUnion::Type(tipe, row, col) => to_type_report(alloc, filename, tipe, row, col),

        TTagUnion::IndentOpen(row, col) => {
            let surroundings = Region::from_rows_cols(start_row, start_col, row, col);
            let region = Region::from_row_col(row, col);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I just started parsing a tag union type, but I got stuck here:"),
                alloc.region_with_subregion(surroundings, region),
                alloc.concat(vec![
                    alloc.reflow(r"Tag unions look like "),
                    alloc.parser_suggestion("[ Many I64, None ],"),
                    alloc.reflow(" so I was expecting to see a tag name next."),
                ]),
                note_for_tag_union_type_indent(alloc),
            ]);

            Report {
                filename,
                doc,
                title: "UNFINISHED TAG UNION TYPE".to_string(),
            }
        }

        TTagUnion::IndentEnd(row, col) => {
            match next_line_starts_with_close_square_bracket(alloc.src_lines, row - 1) {
                Some((curly_row, curly_col)) => {
                    let surroundings =
                        Region::from_rows_cols(start_row, start_col, curly_row, curly_col);
                    let region = Region::from_row_col(curly_row, curly_col);

                    let doc = alloc.stack(vec![
                        alloc.reflow(
                            "I am partway through parsing a tag union type, but I got stuck here:",
                        ),
                        alloc.region_with_subregion(surroundings, region),
                        alloc.concat(vec![
                            alloc.reflow("I need this square bracket to be indented more. Try adding more spaces before it!"),
                        ]),
                    ]);

                    Report {
                        filename,
                        doc,
                        title: "NEED MORE INDENTATION".to_string(),
                    }
                }
                None => {
                    let surroundings = Region::from_rows_cols(start_row, start_col, row, col);
                    let region = Region::from_row_col(row, col);

                    let doc = alloc.stack(vec![
                        alloc.reflow(
                            r"I am partway through parsing a tag union type, but I got stuck here:",
                        ),
                        alloc.region_with_subregion(surroundings, region),
                        alloc.concat(vec![
                            alloc.reflow("I was expecting to see a closing square "),
                            alloc.reflow("bracket before this, so try adding a "),
                            alloc.parser_suggestion("]"),
                            alloc.reflow(" and see if that helps?"),
                        ]),
                        note_for_tag_union_type_indent(alloc),
                    ]);

                    Report {
                        filename,
                        doc,
                        title: "UNFINISHED TAG UNION TYPE".to_string(),
                    }
                }
            }
        }

        TTagUnion::Space(error, row, col) => to_space_report(alloc, filename, &error, row, col),
    }
}

fn to_tinparens_report<'a>(
    alloc: &'a RocDocAllocator<'a>,
    filename: PathBuf,
    parse_problem: &roc_parse::parser::TInParens<'a>,
    start_row: Row,
    start_col: Col,
) -> Report<'a> {
    use roc_parse::parser::TInParens;

    match *parse_problem {
        TInParens::Open(row, col) => {
            match what_is_next(alloc.src_lines, row, col) {
                Next::Keyword(keyword) => {
                    let surroundings = Region::from_rows_cols(start_row, start_col, row, col);
                    let region = to_keyword_region(row, col, keyword);

                    let doc = alloc.stack(vec![
                    alloc.reflow(r"I just saw an open parenthesis, so I was expecting to see a type next."),
                    alloc.region_with_subregion(surroundings, region),
                    alloc.concat(vec![
                        alloc.reflow(r"Something like "),
                        alloc.parser_suggestion("(List Person)"),
                        alloc.text(" or "),
                        alloc.parser_suggestion("(Result I64 Str)"),
                    ]),
                ]);

                    Report {
                        filename,
                        doc,
                        title: "UNFINISHED PARENTHESES".to_string(),
                    }
                }
                Next::Other(Some(c)) if c.is_alphabetic() => {
                    debug_assert!(c.is_lowercase());

                    let surroundings = Region::from_rows_cols(start_row, start_col, row, col);
                    let region = Region::from_row_col(row, col);

                    let doc = alloc.stack(vec![
                    alloc.reflow(
                        r"I am partway through parsing a type in parentheses, but I got stuck here:",
                    ),
                    alloc.region_with_subregion(surroundings, region),
                    alloc.reflow(r"I was expecting to see a tag name."),
                    hint_for_tag_name(alloc),
                ]);

                    Report {
                        filename,
                        doc,
                        title: "WEIRD TAG NAME".to_string(),
                    }
                }
                _ => {
                    let surroundings = Region::from_rows_cols(start_row, start_col, row, col);
                    let region = Region::from_row_col(row, col);

                    let doc = alloc.stack(vec![
                        alloc.reflow(
                            r"I just started parsing a type in parentheses, but I got stuck here:",
                        ),
                        alloc.region_with_subregion(surroundings, region),
                        alloc.concat(vec![
                            alloc.reflow(r"Tag unions look like "),
                            alloc.parser_suggestion("[ Many I64, None ],"),
                            alloc.reflow(" so I was expecting to see a tag name next."),
                        ]),
                    ]);

                    Report {
                        filename,
                        doc,
                        title: "UNFINISHED PARENTHESES".to_string(),
                    }
                }
            }
        }

        TInParens::End(row, col) => {
            let surroundings = Region::from_rows_cols(start_row, start_col, row, col);
            let region = Region::from_row_col(row, col);

            match what_is_next(alloc.src_lines, row, col) {
                Next::Other(Some(c)) if c.is_alphabetic() => {
                    debug_assert!(c.is_lowercase());

                    // TODO hint for tuples?
                    let doc = alloc.stack(vec![
                        alloc.reflow(
                            r"I am partway through parsing a type in parentheses, but I got stuck here:",
                        ),
                        alloc.region_with_subregion(surroundings, region),
                        alloc.reflow(r"I was expecting to see a tag name."),
                        hint_for_tag_name(alloc),
                    ]);

                    Report {
                        filename,
                        doc,
                        title: "WEIRD TAG NAME".to_string(),
                    }
                }
                _ => {
                    let doc = alloc.stack(vec![
                        alloc.reflow(r"I am partway through parsing a type in parentheses, but I got stuck here:"),
                        alloc.region_with_subregion(surroundings, region),
                        alloc.concat(vec![
                                alloc.reflow(
                                    r"I was expecting to see a closing parenthesis before this, so try adding a ",
                                ),
                                alloc.parser_suggestion(")"),
                                alloc.reflow(" and see if that helps?"),
                            ]),
                        ]);

                    Report {
                        filename,
                        doc,
                        title: "UNFINISHED PARENTHESES".to_string(),
                    }
                }
            }
        }

        TInParens::Type(tipe, row, col) => to_type_report(alloc, filename, tipe, row, col),

        TInParens::IndentOpen(row, col) => {
            let surroundings = Region::from_rows_cols(start_row, start_col, row, col);
            let region = Region::from_row_col(row, col);

            let doc = alloc.stack(vec![
                alloc
                    .reflow(r"I just started parsing a type in parentheses, but I got stuck here:"),
                alloc.region_with_subregion(surroundings, region),
                alloc.concat(vec![
                    alloc.reflow(r"Tag unions look like "),
                    alloc.parser_suggestion("[ Many I64, None ],"),
                    alloc.reflow(" so I was expecting to see a tag name next."),
                ]),
                note_for_tag_union_type_indent(alloc),
            ]);

            Report {
                filename,
                doc,
                title: "UNFINISHED PARENTHESES".to_string(),
            }
        }

        TInParens::IndentEnd(row, col) => {
            match next_line_starts_with_close_square_bracket(alloc.src_lines, row - 1) {
                Some((curly_row, curly_col)) => {
                    let surroundings =
                        Region::from_rows_cols(start_row, start_col, curly_row, curly_col);
                    let region = Region::from_row_col(curly_row, curly_col);

                    let doc = alloc.stack(vec![
                        alloc.reflow(
                            "I am partway through parsing a type in parentheses, but I got stuck here:",
                        ),
                        alloc.region_with_subregion(surroundings, region),
                        alloc.concat(vec![
                            alloc.reflow("I need this square bracket to be indented more. Try adding more spaces before it!"),
                        ]),
                    ]);

                    Report {
                        filename,
                        doc,
                        title: "NEED MORE INDENTATION".to_string(),
                    }
                }
                None => {
                    let surroundings = Region::from_rows_cols(start_row, start_col, row, col);
                    let region = Region::from_row_col(row, col);

                    let doc = alloc.stack(vec![
                        alloc.reflow(
                            r"I am partway through parsing a type in parentheses, but I got stuck here:",
                        ),
                        alloc.region_with_subregion(surroundings, region),
                        alloc.concat(vec![
                            alloc.reflow("I was expecting to see a closing square "),
                            alloc.reflow("bracket before this, so try adding a "),
                            alloc.parser_suggestion("]"),
                            alloc.reflow(" and see if that helps?"),
                        ]),
                        note_for_tag_union_type_indent(alloc),
                    ]);

                    Report {
                        filename,
                        doc,
                        title: "UNFINISHED PARENTHESES".to_string(),
                    }
                }
            }
        }

        TInParens::Space(error, row, col) => to_space_report(alloc, filename, &error, row, col),
    }
}

fn to_tapply_report<'a>(
    alloc: &'a RocDocAllocator<'a>,
    filename: PathBuf,
    parse_problem: &roc_parse::parser::TApply,
    _start_row: Row,
    _start_col: Col,
) -> Report<'a> {
    use roc_parse::parser::TApply;

    match *parse_problem {
        TApply::DoubleDot(row, col) => {
            let region = Region::from_row_col(row, col);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I encountered two dots in a row:"),
                alloc.region(region),
                alloc.concat(vec![alloc.reflow("Try removing one of them.")]),
            ]);

            Report {
                filename,
                doc,
                title: "DOUBLE DOT".to_string(),
            }
        }
        TApply::TrailingDot(row, col) => {
            let region = Region::from_row_col(row, col);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I encountered a dot with nothing after it:"),
                alloc.region(region),
                alloc.concat(vec![
                    alloc.reflow("Dots are used to refer to a type in a qualified way, like "),
                    alloc.parser_suggestion("Num.I64"),
                    alloc.text(" or "),
                    alloc.parser_suggestion("List.List a"),
                    alloc.reflow(". Try adding a type name next."),
                ]),
            ]);

            Report {
                filename,
                doc,
                title: "TRAILING DOT".to_string(),
            }
        }
        TApply::StartIsNumber(row, col) => {
            let region = Region::from_row_col(row, col);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I encountered a number at the start of a qualified name segment:"),
                alloc.region(region),
                alloc.concat(vec![
                    alloc.reflow("All parts of a qualified type name must start with an uppercase letter, like "),
                    alloc.parser_suggestion("Num.I64"),
                    alloc.text(" or "),
                    alloc.parser_suggestion("List.List a"),
                    alloc.text("."),
                ]),
            ]);

            Report {
                filename,
                doc,
                title: "WEIRD QUALIFIED NAME".to_string(),
            }
        }
        TApply::StartNotUppercase(row, col) => {
            let region = Region::from_row_col(row, col);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I encountered a lowercase letter at the start of a qualified name segment:"),
                alloc.region(region),
                alloc.concat(vec![
                    alloc.reflow("All parts of a qualified type name must start with an uppercase letter, like "),
                    alloc.parser_suggestion("Num.I64"),
                    alloc.text(" or "),
                    alloc.parser_suggestion("List.List a"),
                    alloc.text("."),
                ]),
            ]);

            Report {
                filename,
                doc,
                title: "WEIRD QUALIFIED NAME".to_string(),
            }
        }

        TApply::End(row, col) => {
            let region = Region::from_row_col(row, col);

            let doc = alloc.stack(vec![
                alloc.reflow(
                    r"I reached the end of the input file while parsing a qualified type name",
                ),
                alloc.region(region),
            ]);

            Report {
                filename,
                doc,
                title: "END OF FILE".to_string(),
            }
        }

        TApply::Space(error, row, col) => to_space_report(alloc, filename, &error, row, col),
    }
}

fn to_space_report<'a>(
    alloc: &'a RocDocAllocator<'a>,
    filename: PathBuf,
    parse_problem: &roc_parse::parser::BadInputError,
    row: Row,
    col: Col,
) -> Report<'a> {
    use roc_parse::parser::BadInputError;

    match parse_problem {
        BadInputError::HasTab => {
            let region = Region::from_row_col(row, col);

            let doc = alloc.stack(vec![
                alloc.reflow(r"I encountered a tab character"),
                alloc.region(region),
                alloc.concat(vec![alloc.reflow("Tab characters are not allowed.")]),
            ]);

            Report {
                filename,
                doc,
                title: "TAB CHARACTER".to_string(),
            }
        }
        _ => todo!("unhandled type parse error: {:?}", &parse_problem),
    }
}

enum Next<'a> {
    Keyword(&'a str),
    // Operator(&'a str),
    Close(&'a str, char),
    Other(Option<char>),
}

fn what_is_next<'a>(source_lines: &'a [&'a str], row: Row, col: Col) -> Next<'a> {
    let row_index = row as usize;
    let col_index = col as usize;
    match source_lines.get(row_index) {
        None => Next::Other(None),
        Some(line) => {
            let chars = &line[col_index..];

            match roc_parse::keyword::KEYWORDS
                .iter()
                .find(|keyword| starts_with_keyword(chars, keyword))
            {
                Some(keyword) => Next::Keyword(keyword),
                None => match chars.chars().next() {
                    None => Next::Other(None),
                    Some(c) => match c {
                        ')' => Next::Close("parenthesis", ')'),
                        ']' => Next::Close("square bracket", ']'),
                        '}' => Next::Close("curly brace", '}'),
                        // _ if is_symbol(c) => todo!("it's an operator"),
                        _ => Next::Other(Some(c)),
                    },
                },
            }
        }
    }
}

fn starts_with_keyword(rest_of_line: &str, keyword: &str) -> bool {
    if let Some(stripped) = rest_of_line.strip_prefix(keyword) {
        match stripped.chars().next() {
            None => true,
            Some(c) => !c.is_alphanumeric(),
        }
    } else {
        false
    }
}

fn next_line_starts_with_close_curly(source_lines: &[&str], row: Row) -> Option<(Row, Col)> {
    match source_lines.get(row as usize + 1) {
        None => None,

        Some(line) => {
            let spaces_dropped = line.trim_start_matches(' ');
            match spaces_dropped.chars().next() {
                Some('}') => Some((row + 1, (line.len() - spaces_dropped.len()) as u16)),
                _ => None,
            }
        }
    }
}

fn next_line_starts_with_close_square_bracket(
    source_lines: &[&str],
    row: Row,
) -> Option<(Row, Col)> {
    match source_lines.get(row as usize + 1) {
        None => None,

        Some(line) => {
            let spaces_dropped = line.trim_start_matches(' ');
            match spaces_dropped.chars().next() {
                Some(']') => Some((row + 1, (line.len() - spaces_dropped.len()) as u16)),
                _ => None,
            }
        }
    }
}

fn to_keyword_region(row: Row, col: Col, keyword: &str) -> Region {
    Region {
        start_line: row,
        start_col: col,
        end_line: row,
        end_col: col + keyword.len() as u16,
    }
}
