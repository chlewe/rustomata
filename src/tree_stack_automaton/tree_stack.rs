use std::cmp::Ordering;
use std::rc::Rc;
use std::hash::Hash;
use util::integerisable::Integerisable1;
use integeriser::{HashIntegeriser, Integeriser};

/// upside-down tree with a designated position (the *stack pointer*) and *nodes* of type `a`.
#[derive(Debug, Clone)]
pub struct TreeStack<A> {
    parent: Option<(usize, Rc<TreeStack<A>>)>,
    value: A,
    children: Vec<Option<Rc<TreeStack<A>>>>,
}

impl<A> TreeStack<A> {
    /// Creates a new `TreeStack<A>` with root label `a`.
    pub fn new(a: A) -> Self {
        TreeStack { value: a, children: Vec::new(), parent: None }
    }

    /// Applies a function `Fn(&A) -> B`to every node in a `TreeStack<A>`.
    pub fn map<B>(&self, f: &Fn(&A) -> B) -> TreeStack<B> {
        let new_parent = match self.parent {
            Some((i, ref p)) => Some((i, Rc::new(p.map(f)))),
            None => None,
        };
        let new_children = self.children.iter().map(|o| o.clone().map(|v| Rc::new(v.map(f)))).collect();
        TreeStack { parent: new_parent, value: f(&self.value), children: new_children }
    }

    /// Applies a function `FnMut(&A) -> B`to every node in a `TreeStack<A>`.
    pub fn map_mut<B: Clone>(&self, f: &mut FnMut(&A) -> B) -> TreeStack<B> {
        let new_parent = match self.parent {
            Some((i, ref p)) => Some((i, Rc::new(p.map_mut(f)))),
            None => None,
        };
        let new_children = self.children.iter().map(|o| o.clone().map(|v| Rc::new(v.map_mut(f)))).collect();
        TreeStack { parent: new_parent, value: f(&self.value), children: new_children }
    }

    /// Returns `True` if the stack pointer points to the bottom node.
    pub fn is_at_bottom(&self) -> bool {
        self.parent.is_none()
    }

    /// Returns a reference to label of the current node.
    pub fn current_symbol(&self) -> &A {
        &self.value
    }

    /// Replaces the current value by the given value.
    pub fn set(mut self, a: A) -> Self {
        self.value = a;
        self
    }

    /// Writes a value to the specified child position (if the child position is vacant) and returns the resulting `TreeStack` in an `Ok`.
    /// Returns the unmodified `TreeStack` in an `Err` otherwise.
    pub fn push(mut self, n: usize, a: A) -> Result<Self, Self> {
        if n >= self.children.len() {
            let len = n - self.children.len() + 1;
            let filler = &mut vec![None; len];
            self.children.append(filler);
        }

        if self.children[n].is_none() {
            Ok(TreeStack { value: a,
                           children: Vec::new(),
                           parent: Some((n, Rc::new(self))) })
        } else {
            Err(self)
        }
    }
}

impl<A: Clone> TreeStack<A> {
    /// Goes up to a specific child position (if this position is occupied) and returns the resulting `TreeStack` in an `Ok`.
    /// Returns the unmodified `TreeStack` in an `Err` otherwise.
    pub fn up(mut self, n: usize) -> Result<Self, Self> {
        match {
            if self.children.len() > n {
                self.children.push(None);
                self.children.swap_remove(n)
            } else {
                None
            }
        } {
            Some(ref tn) => Ok(TreeStack { value: tn.value.clone(),
                                           children: tn.children.clone(),
                                           parent: Some((n, Rc::new(self))) }),
            _ => Err(self),
        }
    }

    /// Goes down to the parent position (if there is one) and returns the resulting `TreeStack` in an `Ok`.
    /// Returns the unmodified `TreeStack` in an `Err` otherwise.
    pub fn down(mut self) -> Result<Self, Self> {
        match self.parent.take() {
            Some((n, pn)) => {
                let mut new_pch = pn.children.clone();
                new_pch[n] = Some(Rc::new(self));
                Ok(TreeStack { value: pn.value.clone(),
                               children: new_pch,
                               parent: pn.parent.clone() })
            },
            None => Err(self),
        }
    }
}


impl<A: Clone + Eq + Hash> Integerisable1 for TreeStack<A> {
    type AInt = TreeStack<usize>;
    type I = HashIntegeriser<A>;

    fn integerise(&self, integeriser: &mut Self::I) -> Self::AInt {
        self.map_mut(&mut move |v| integeriser.integerise(v.clone()))
    }

    fn un_integerise(aint: &Self::AInt, integeriser: &Self::I) -> Self {
        aint.map(&|&v| integeriser.find_value(v).unwrap().clone())
    }
}


impl<A: PartialEq> PartialEq for TreeStack<A> {
    fn eq(&self, other: &Self) -> bool {
        let comp = |p1, p2| Rc::ptr_eq(p1, p2) || p1 == p2;
        self.value == other.value
            && match (&self.parent, &other.parent) {
                (&Some((i1, ref p1)), &Some((i2, ref p2))) => i1 == i2 && comp(p1, p2),
                (&None, &None)                             => true,
                _                                          => false,
            }
            && self.children == other.children
    }
}

impl<A: Eq> Eq for TreeStack<A> {}

impl<A: PartialOrd> PartialOrd for TreeStack<A> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.value.partial_cmp(&other.value) {
            None | Some(Ordering::Equal) =>
                self.children.partial_cmp(&other.children),
            x => x,
        }
    }
}

impl<A: Ord> Ord for TreeStack<A> {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.value.cmp(&other.value) {
            Ordering::Equal =>
                self.children.cmp(&other.children),
            x => x,
        }
    }
}

#[test]
fn test_tree_stack() {
    let mut ts: TreeStack<u8> = TreeStack::new(0);
    assert_eq!(&0, ts.current_symbol());

    ts = ts.push(1, 1).unwrap().clone();
    assert_eq!(&1, ts.current_symbol());

    ts = ts.down().unwrap().clone();
    assert_eq!(&0, ts.current_symbol());

    ts = ts.push(2, 2).unwrap().clone();
    assert_eq!(&2, ts.current_symbol());

    ts = ts.down().unwrap().clone();
    ts = ts.up(1).unwrap().clone();
    assert_eq!(&1, ts.current_symbol());

    let ts1 = ts.clone().push(1, 11);
    let ts2 = ts.push(1, 11);
    assert_eq!(ts1, ts2);
}
