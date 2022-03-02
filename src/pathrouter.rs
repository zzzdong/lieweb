#[derive(Debug, PartialEq)]
enum Pattern {
    Root,
    Static(String),
    Param(String),
    Any(String),
}

struct Node {
    index: usize,
    pattern: Pattern,
    children: Vec<usize>,
}

pub struct Tree {
    nodes: Vec<Node>,
}

impl Tree {
    pub fn new() -> Tree {
        let root = Node {
            index: 0,
            pattern: Pattern::Root,
            children: Vec::new(),
        };

        Tree { nodes: vec![root] }
    }

    pub fn insert(&mut self, path: &str) {
        let mut root = self.nodes.first().unwrap();
    }

    fn node_add_child(&mut self, node: &mut Node, child: Node) {
        if node.pattern != child.pattern {
            return;
        }

        match (&node.pattern, &child.pattern) {
            (Pattern::Static(p), Pattern::Static(ref c)) => {
                if p == c {
                    node.children.push(child.index);
                } else {

                }

            }
            _ => {
                unreachable!()
            }
        }
    }
}

fn find_commprefix<'s>(a: &'s str, b: &'s str) -> &'s str {
    let mut offset: usize = 0;
    let mut aa = a.chars();
    let mut bb = b.chars();

    loop {
        match (aa.next(), bb.next()) {
            (Some(_a), None) => {
                return &a[..offset];
            }

            (None, Some(_b)) => {
                return &b[..offset];
            }

            (None, None) => {
                return &a[..offset];
            }

            (Some(ac), Some(bc)) => {
                if ac == bc {
                    offset += 1;
                    continue;
                }

                return &a[..offset];
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_router() {
        let s = vec![
            ("abc", "abc", "abc"),
            ("", "", ""),
            ("abc", "", ""),
            ("", "abc", ""),
            ("0", "abc", ""),
            ("abc", "0", ""),
            ("abchijk", "abcdefg", "abc"),
            ("abcdefg", "abchijklmn", "abc"),
        ];

        for item in &s {
            println!("{:?} => {:?}", &item, find_commprefix(item.0, item.1));
            assert_eq!(find_commprefix(item.0, item.1), item.2)
        }
    }
}
