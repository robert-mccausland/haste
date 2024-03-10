use std::fmt::Debug;

#[derive(Debug)]
pub struct Node<T> {
    pub weight: u32,
    pub content: NodeContent<T>,
    precedence: usize,
}

#[derive(Debug)]
pub enum NodeContent<T> {
    Leaf(T),
    Parent(Box<Node<T>>, Box<Node<T>>),
}

pub trait Weighted {
    fn weight(&self) -> u32;
}

pub fn build_tree<T>(items: Vec<T>) -> Node<T>
where
    T: Clone + Weighted,
{
    assert_ne!(items.len(), 0);

    let order = |left: &Node<T>, right: &Node<T>| {
        if left.weight == right.weight {
            left.precedence.cmp(&right.precedence).reverse()
        } else {
            left.weight.cmp(&right.weight)
        }
    };

    let mut nodes = items
        .into_iter()
        .enumerate()
        .map(|(i, item)| Node {
            weight: item.weight(),
            precedence: i,
            content: NodeContent::Leaf(item),
        })
        .collect::<Vec<_>>();
    nodes.sort_by(order);

    let mut n = nodes.len();
    loop {
        if nodes.len() == 1 {
            return nodes.pop().unwrap();
        }

        let left = nodes.remove(0);
        let right = nodes.remove(0);
        let node = Node {
            weight: left.weight + right.weight,
            precedence: n,
            content: NodeContent::Parent(Box::new(left), Box::new(right)),
        };

        match nodes.binary_search_by(|element| order(element, &node)) {
            Ok(_) => {
                panic!("elements should not have the same ordering")
            }
            Err(index) => nodes.insert(index, node),
        }

        n += 1;
    }
}
