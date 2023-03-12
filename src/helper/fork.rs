use std::collections::{btree_map, BTreeMap};

macro_rules! chaotic {
    ($min_size:expr;$($key:expr => $value:expr),* $(,)?) => {
        Fork::Chaotic {
            min_size: $min_size,
            items: BTreeMap::from_iter([
                $(($key, $value)),*
            ])
        }
    };
}

macro_rules! run {
    ($prefix:expr,$min_size:expr; $remaining:expr) => {
        Fork::Run {
            prefix: $prefix,
            min_size: $min_size,
            remaining: Box::new($remaining),
        }
    };
}

macro_rules! empty {
    (($total_prefix:expr,$value:expr);$remaining:expr) => {
        Fork::Empty {
            total_prefix: $total_prefix,
            value: $value,
            remaining: Box::new($remaining),
        }
    };
}

macro_rules! end {
    ($prefix:expr; ($total_prefix:expr,$value:expr)) => {
        Fork::End {
            prefix: $prefix,
            total_prefix: $total_prefix,
            value: $value,
        }
    };
}

#[derive(Default, PartialEq, Debug)]
/// This is a helper data-structure to build the `getDataIndex` function.
/// It's very similar to a [Radix Trie](https://en.wikipedia.org/wiki/Radix_tree), though here it's changed
/// to simplify printing the function.
// TODO: remove these bounds
pub enum Fork<'a, V: std::fmt::Debug + PartialEq + Copy> {
    /// All children of this fork have the same prefix.
    ///
    /// ### Example
    ///
    /// Here, we have a `Chaotic` ('o', 'a', 'k'),
    /// and a `Run` (with `pple` as prefix) followed by another `Chaotic` ('j', 'l').
    ///
    /// ```text
    ///         Run
    ///          ▾
    ///          │
    ///   ╭─o..  │
    ///   │   ╭──┴╮
    /// >─┼─a─pple─┬─j..
    ///   │        ╰─l..
    ///   ╰─k..    
    /// ```
    Run {
        /// The prefix for this run (the common prefix of all children).
        prefix: &'a [u8],
        /// The length of the shortest item inside this fork.
        min_size: usize,
        /// The children after this run.
        remaining: Box<Fork<'a, V>>,
    },
    /// At this fork, all children start with a different prefix.
    ///
    /// ### Example
    ///
    /// Here, we have a [`Chaotic`](Self::Chaotic) ('o', 'a', 'k') where all child-forks are [`End`](Self::End)s
    ///
    /// ```text
    ///   Chaotic
    ///      ▾
    ///  ╭───┤
    ///   ╭─o─range
    ///   │     
    /// >─┼─a─pple
    ///   │
    ///   ╰─k─iwi
    /// ```
    Chaotic {
        /// The length of the shortest item inside this fork.
        min_size: usize,
        // we're using a BTreeMap here to keep the ouput sorted
        // (avoids recompilations at the cost of speed)
        /// All children of this fork.
        items: BTreeMap<u8, Fork<'a, V>>,
    },
    /// At this fork an item ends. This item has the key of `total_prefix` and value of `value`.
    ///
    /// ### Example
    ///
    /// Here, we have a [`Run`](Self::Run) ('apple') followed by an [`Empty`](Self::Empty),
    /// where the remainder is an [`End`](Self::End).
    ///
    /// This is the resulting fork after inserting `apple` and `applejuice`.
    ///
    /// ```text
    ///          Empty
    ///            ▾
    ///         ╭──┤
    /// >──apple─┬─▧
    ///          ╰─juice
    /// ```
    Empty {
        total_prefix: &'a [u8],
        value: V,
        remaining: Box<Fork<'a, V>>,
    },
    /// This fork is a terminator. The item has the key of `total_prefix` and value of `value`.
    ///
    /// ### Example
    ///
    /// Here, we have a [`Run`](Self::Run) ('apple') followed by an [`Empty`](Self::Empty),
    /// where the remainder is an [`End`](Self::End).
    ///
    /// This is the resulting fork after inserting `apple` and `applejuice`.
    ///
    /// ```text
    ///               End
    ///                ▾
    ///            ╭───┤
    /// >──apple─┬─juice
    ///          ╰─▧
    /// ```
    End {
        prefix: &'a [u8],
        total_prefix: &'a [u8],
        value: V,
    },
    /// This fork is an empty fork. No items have been inserted yet.
    #[default]
    None,
}

impl<'a, V> Fork<'a, V>
where
    V: Default + Copy + PartialEq + std::fmt::Debug,
{
    /// Creates a new fork with no items in it.
    ///
    /// This doesn't allocate.
    pub fn new() -> Self {
        Self::None
    }

    /// Returns true iff this fork is of type [`Empty`](Self::Empty).
    fn is_empty(&self) -> bool {
        matches!(self, Fork::Empty { .. })
    }

    /// Inserts a new item with `key` and `value` into this fork.
    pub fn insert(&mut self, key: &'a [u8], value: V) {
        self.insert_inner(key, key, value);
    }

    /// Returns the size of the smallest key inside this fork.
    ///
    /// This is done in constant time.
    pub fn min_size(&self) -> usize {
        match self {
            Fork::Run { min_size, .. } => *min_size,
            Fork::Chaotic { min_size, .. } => *min_size,
            Fork::Empty { total_prefix, .. } => total_prefix.len(),
            Fork::End { total_prefix, .. } => total_prefix.len(),
            Fork::None => 0,
        }
    }

    /// Recursively called function that attempts to insert.
    ///
    /// `key` is the suffix of the `total_key` relevant for this fork.
    fn insert_inner(&mut self, key: &'a [u8], total_key: &'a [u8], value: V) {
        match self {
            Fork::Run {
                prefix,
                remaining,
                min_size,
            } => match match_prefix(prefix, key) {
                // `prefix` and `key` have a different first letter.
                // => switch to chaotic
                PrefixMatch::Run(n) if n == 0 => {
                    *self = chaotic!((*min_size).min(total_key.len());
                        prefix[0] => if prefix.len() > 1 {
                                Fork::Run {
                                    min_size: *min_size,
                                    prefix: &prefix[1..],
                                    remaining: std::mem::take(remaining),
                                }
                            } else {
                                std::mem::take(remaining.as_mut())
                            },
                        key[0] => end!(&key[1..]; (total_key, value))
                    );
                }
                // `prefix` and `key` both have a matching prefix,
                // but after `n` characters, they're different.
                // => continue to use the matching prefix,
                // use a Chaotic when they're different
                PrefixMatch::Run(n) => {
                    *self = run!(&prefix[..n], (*min_size).min(total_key.len());
                        chaotic!(
                            (*min_size).min(total_key.len());
                            prefix[n] =>
                                if prefix.len() != n + 1 {
                                    Fork::Run {
                                        prefix: &prefix[(n + 1)..],
                                        min_size: *min_size,
                                        remaining: std::mem::take(
                                            remaining,
                                        ),
                                    }
                                } else {
                                    std::mem::take(remaining.as_mut())
                                },
                            key[n] => end!(&key[(n + 1)..]; (total_key, value))
                        )
                    );
                }
                // `key` starts with `prefix` but has characters after it.
                // => strip prefix and insert in the remaining fork
                PrefixMatch::LeftInRight => {
                    *min_size = (*min_size).min(total_key.len());
                    remaining.insert_inner(
                        &key[prefix.len()..],
                        total_key,
                        value,
                    );
                }
                // `key` is empty.
                // => insert an empty item - wrapping `self`
                PrefixMatch::RightInLeft if key.is_empty() => {
                    *self = empty!((total_key, value); std::mem::take(self));
                }
                // `run` starts with `key`, but `run` is longer
                // => split into two runs and one empty
                //    run { empty; run{remaining} }
                PrefixMatch::RightInLeft => {
                    *self = run!(&prefix[..key.len()], (*min_size).min(total_key.len());
                        empty!((total_key, value);
                            Fork::Run {
                                min_size: *min_size,
                                prefix: &prefix[key.len()..],
                                remaining: std::mem::take(remaining),
                            }
                        )
                    );
                }
                // `key` == `prefix` and remaining == Empty {..}
                // replace value in remaining
                PrefixMatch::Equal if remaining.is_empty() => {
                    remaining.insert_inner(&[], total_key, value);
                }
                // `key` == `prefix` and remaining != Empty {..}
                // => insert an empty
                PrefixMatch::Equal => {
                    *min_size = (*min_size).min(total_key.len());
                    *remaining = Box::new(Fork::Empty {
                        total_prefix: total_key,
                        value,
                        remaining: std::mem::take(remaining),
                    });
                }
            },
            // `key` is empty => insert an empty here
            Fork::Chaotic { items, min_size } if key.is_empty() => {
                *self = empty!((total_key, value);
                    Fork::Chaotic {
                        min_size: *min_size,
                        items: std::mem::take(items),
                    }
                );
            }
            // `key` isn't empty
            // => find/insert the slot where `key[0]` fits
            Fork::Chaotic { items, min_size } => match items.entry(key[0]) {
                // the slot already exists
                //  => insert there
                btree_map::Entry::Occupied(mut e) => {
                    *min_size = (*min_size).min(total_key.len());
                    e.get_mut().insert_inner(&key[1..], total_key, value);
                }
                // the slot doesn't exist yet
                // => create a new slot and an end
                btree_map::Entry::Vacant(e) => {
                    *min_size = (*min_size).min(total_key.len());
                    e.insert(end!(&key[1..]; (total_key, value)));
                }
            },
            Fork::End {
                prefix,
                total_prefix,
                value: end_value,
            } => match match_prefix(prefix, key) {
                // `prefix`'s and `key`'s first characters are different
                // (implicitly: `prefix != "" && key != ""`)
                // => insert a chaotic
                PrefixMatch::Run(n) if n == 0 => {
                    *self = chaotic!(
                        total_prefix.len().min(total_key.len());
                        prefix[0] => end!(&prefix[1..]; (total_prefix, *end_value)),
                        key[0] => end!(&key[1..]; (total_key, value))
                    );
                }
                // `prefix` and `key` match for the first `n` characters,
                // but they're not equal
                // => insert a run followed by a chaotic
                PrefixMatch::Run(n) => {
                    *self = run!(&prefix[..n], total_prefix.len().min(total_key.len());
                        chaotic!(total_prefix.len().min(total_key.len());
                            prefix[n] => end!(&prefix[(n + 1)..]; (total_prefix, *end_value)),
                            key[n] => end!(&key[(n+1)..]; (total_key, value))
                        )
                    );
                }
                // `prefix` is empty
                // => insert an empty followed by an end
                PrefixMatch::LeftInRight if prefix.is_empty() => {
                    *self = empty!((total_prefix, *end_value); end!(key; (total_key, value)));
                }
                // `key` starts with `prefix`
                // => insert a run of `prefix` followed by an empty and end
                PrefixMatch::LeftInRight => {
                    *self = run!(prefix, total_prefix.len().min(total_key.len());
                        empty!((total_prefix, *end_value);
                            end!(&key[prefix.len()..]; (total_key, value))
                        )
                    );
                }
                // `key` is empty
                // => insert an empty followed by this end
                PrefixMatch::RightInLeft if key.is_empty() => {
                    *self = empty!((total_key, value); end!(prefix; (total_prefix, *end_value)));
                }
                // `prefix` starts with `key` (similar to ::LeftInRight)
                // => insert a run of `key` followed by an empty and an end
                PrefixMatch::RightInLeft => {
                    *self = run!(key, total_prefix.len().min(total_key.len());
                        empty!((total_key, value);
                            end!(&prefix[key.len()..]; (total_prefix, *end_value))
                        )
                    );
                }
                // => replace the value
                PrefixMatch::Equal => {
                    *end_value = value;
                }
            },
            Fork::Empty {
                remaining,
                value: empty_value,
                ..
            } => {
                if key.is_empty() {
                    // both are equal,
                    // => replace the value
                    *empty_value = value;
                } else {
                    // continue inserting in the remaining fork
                    remaining.insert_inner(key, total_key, value)
                }
            }
            Fork::None => {
                // this is the first item
                *self = end!(key; (total_key, value));
            }
        }
    }
}

enum PrefixMatch {
    Run(usize),
    LeftInRight,
    RightInLeft,
    Equal,
}
fn match_prefix(left: &[u8], right: &[u8]) -> PrefixMatch {
    for i in 0..left.len().min(right.len()) {
        if left[i] != right[i] {
            return PrefixMatch::Run(i);
        }
    }

    match (left.len(), right.len()) {
        (l, r) if l > r => PrefixMatch::RightInLeft,
        (l, r) if l < r => PrefixMatch::LeftInRight,
        _ => PrefixMatch::Equal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_insert() {
        let mut fork = Fork::default();
        fork.insert(b"forsen", 1);
        fork.insert(b"xqc", 2);

        assert_eq!(
            fork,
            chaotic!(3;
                b'f' => end!(b"orsen";(b"forsen", 1)),
                b'x' => end!(b"qc";(b"xqc", 2))
            )
        );

        fork.insert(b"forsenL", 3);
        assert_eq!(
            fork,
            chaotic!(3;
                b'f' => run!(b"orsen", 6;
                            empty!((b"forsen", 1);
                                end!(b"L"; (b"forsenL", 3))
                            )
                        ),
                b'x' => end!(b"qc"; (b"xqc", 2))
            )
        );

        fork.insert(b"for", 4);
        assert_eq!(
            fork,
            chaotic!(3;
                b'f' => run!(b"or", 3;
                            empty!((b"for", 4);
                                run!(b"sen", 6;
                                    empty!((b"forsen", 1);
                                        end!(b"L"; (b"forsenL", 3))
                                    )
                                )
                            )
                        ),
                b'x' => end!(b"qc"; (b"xqc", 2))
            )
        );

        fork.insert(b"forsenE", 5);
        assert_eq!(
            fork,
            chaotic!(
                3;
                b'f' => run!(b"or", 3;
                                empty!((b"for", 4);
                                    run!(b"sen", 6;
                                        empty!((b"forsen", 1);
                                            chaotic!(7;
                                                b'L' => end!(b""; (b"forsenL", 3)),
                                                b'E' => end!(b""; (b"forsenE", 5))
                                            )
                                        )
                                    )
                                )
                            ),
                b'x' => end!(b"qc"; (b"xqc", 2))
            )
        );
    }

    #[test]
    fn insert_run_empty() {
        let mut fork = Fork::new();

        fork.insert(b"applejuice", 1);
        assert_eq!(fork, end!(b"applejuice"; (b"applejuice", 1)));
        fork.insert(b"applepie", 1);
        assert_eq!(
            fork,
            run!(b"apple", 8;
                chaotic!(8;
                    b'j' => end!(b"uice"; (b"applejuice", 1)),
                    b'p' => end!(b"ie"; (b"applepie", 1))
                )
            )
        );
        fork.insert(b"banana", 1);
        assert_eq!(
            fork,
            chaotic!(6;
                b'a' => run!(b"pple", 8;
                            chaotic!(8;
                                b'j' => end!(b"uice"; (b"applejuice", 1)),
                                b'p' => end!(b"ie"; (b"applepie", 1))
                            )
                        ),
                b'b' => end!(b"anana"; (b"banana", 1))
            )
        );
        fork.insert(b"apple", 1);
        assert_eq!(
            fork,
            chaotic!(5;
                b'a' => run!(b"pple", 5;
                            empty!((b"apple", 1);
                                chaotic!(8;
                                    b'j' => end!(b"uice"; (b"applejuice", 1)),
                                    b'p' => end!(b"ie"; (b"applepie", 1))
                                )
                            )
                        ),
                b'b' => end!(b"anana"; (b"banana", 1))
            )
        );
        fork.insert(b"a", 1);
        assert_eq!(
            fork,
            chaotic!(1;
                b'a' => empty!((b"a", 1);
                            run!(b"pple", 5;
                                empty!((b"apple", 1);
                                    chaotic!(8;
                                        b'j' => end!(b"uice"; (b"applejuice", 1)),
                                        b'p' => end!(b"ie"; (b"applepie", 1))
                                    )
                                )
                            )
                        ),
                b'b' => end!(b"anana"; (b"banana", 1))
            )
        );
    }

    #[test]
    fn min_size() {
        let mut fork = Fork::None;
        let input = [
            "colors.accentcolor",
            "messages.backgrounds.regular",
            "messages.backgrounds.alternate",
            "messages.disabled",
            "messages.highlightanimationend",
            "messages.highlightanimationstart",
            "messages.selection",
            "messages.textcolors.regular",
            "messages.textcolors.caret",
            "messages.textcolors.link",
            "messages.textcolors.system",
            "messages.textcolors.chatplaceholder",
            "scrollbars.background",
            "scrollbars.highlights.highlight",
            "scrollbars.highlights.subscription",
            "scrollbars.thumb",
            "scrollbars.thumbselected",
            "splits.background",
            "splits.droppreview",
            "splits.droppreviewborder",
            "splits.droptargetrect",
            "splits.droptargetrectborder",
            "splits.header.border",
            "splits.header.focusedborder",
            "splits.header.background",
            "splits.header.focusedbackground",
            "splits.header.text",
            "splits.header.focusedtext",
            "splits.input.border",
            "splits.input.background",
            "splits.input.selection",
            "splits.input.focusedline",
            "splits.input.text",
            "splits.messageseperator",
            "splits.resizehandle",
            "splits.resizehandlebackground",
            "tabs.border",
            "tabs.dividerline",
            "tabs.highlighted.backgrounds.regular",
            "tabs.highlighted.backgrounds.hover",
            "tabs.highlighted.backgrounds.unfocused",
            "tabs.highlighted.line.regular",
            "tabs.highlighted.line.hover",
            "tabs.highlighted.line.unfocused",
            "tabs.highlighted.text",
            "tabs.newmessage.backgrounds.regular",
            "tabs.newmessage.backgrounds.hover",
            "tabs.newmessage.backgrounds.unfocused",
            "tabs.newmessage.line.regular",
            "tabs.newmessage.line.hover",
            "tabs.newmessage.line.unfocused",
            "tabs.newmessage.text",
            "tabs.regular.backgrounds.regular",
            "tabs.regular.backgrounds.hover",
            "tabs.regular.backgrounds.unfocused",
            "tabs.regular.line.regular",
            "tabs.regular.line.hover",
            "tabs.regular.line.unfocused",
            "tabs.regular.text",
            "tabs.selected.backgrounds.regular",
            "tabs.selected.backgrounds.hover",
            "tabs.selected.backgrounds.unfocused",
            "tabs.selected.line.regular",
            "tabs.selected.line.hover",
            "tabs.selected.line.unfocused",
            "tabs.selected.text",
            "tooltip.text",
            "tooltip.background",
            "window.text",
            "window.background",
            "window.borderunfocused",
            "window.borderfocused",
        ];

        let mut min_size = input[0].len();
        for item in input {
            fork.insert(item.as_bytes(), 1);
            min_size = min_size.min(item.len());
            assert_eq!(min_size, fork.min_size(), "When inserting {item}");
        }
    }
}
