use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt::Debug;

/// A Iterator that run merge sort on `N` ordered iterators.
///
/// This implementation is optimized based on the assumption that most iterators are empty.
/// Compare to first implementation at [rust playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=6919618250c88342b73e5faeb513d212)
pub struct MergeSortIterator<T, I, F> {
    sources: BTreeMap<usize, I>,
    buffered: BTreeMap<usize, T>,
    #[cfg(debug_assertions)]
    last_elements: BTreeMap<usize, T>,
    ordering: Order,
    compare: F,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Order {
    Asc,
    Desc,
}

impl<T, I, F> MergeSortIterator<T, I, F>
where
    T: Clone + Debug,
    I: Iterator<Item = T>,
    F: Fn(&T, &T) -> Ordering,
{
    /// Create a `MergeSortIterator` with a custom comparator.
    pub fn compare_by(sources: Vec<I>, ordering: Order, compare: F) -> Self {
        let (buffered, sources): (BTreeMap<usize, T>, BTreeMap<usize, I>) = sources
            .into_iter()
            .map(|mut iter| (iter.next(), iter))
            .filter(|(next, _iter)| next.is_some())
            .enumerate()
            .map(|(idx, (next, iter))| ((idx, next.unwrap()), (idx, iter)))
            .unzip();
        Self {
            sources,
            buffered,
            #[cfg(debug_assertions)]
            last_elements: BTreeMap::new(),
            ordering,
            compare,
        }
    }

    /// Find the most `ordering` element in the `buffered` array.
    /// ## panics
    /// When the elements in `buffered` are all `None`, calling to this function will panics.
    /// Build a `MergeSortIterator` from a array of Iterator
    pub fn arg_cmp_by(&self) -> usize {
        use Order::*;

        let tmp = self.buffered.iter();
        *match self.ordering {
            Asc => tmp.min_by(|(_idx, x), (_idy, y)| (self.compare)(x, y)),
            Desc => tmp.max_by(|(_idx, x), (_idy, y)| (self.compare)(x, y)),
        }
        .unwrap()
        .0
    }

    /// swap out the element by idx and get next element if possible.
    fn swap_next(&mut self, idx: usize) -> T {
        if let Some(next) = self.sources.get_mut(&idx).unwrap().next() {
            self.buffered.insert(idx, next).unwrap()
        } else {
            self.buffered.remove(&idx).unwrap()
        }
    }

    #[cfg(debug_assertions)]
    /// check ordering
    fn continuation_check(&mut self, idx: usize, new: &T) {
        use Order::*;
        use Ordering::*;

        if self.last_elements.contains_key(&idx)
            && match (self.compare)(&self.last_elements.insert(idx, new.clone()).unwrap(), new) {
                Less => self.ordering == Desc,
                Equal => false,
                Greater => self.ordering == Asc,
            }
        {
            panic!(
                "provided iterator is not ordered, last element: {:?}, current element: {:?}",
                self.last_elements[&idx], new
            )
        }
    }
}

impl<T, I> MergeSortIterator<T, I, fn(&T, &T) -> Ordering>
where
    T: Clone + Debug + Ord,
    I: Iterator<Item = T>,
{
    /// Default comparator when `Ord` trait satisfied
    pub fn new(sources: Vec<I>, ordering: Order) -> Self {
        Self::compare_by(sources, ordering, Ord::cmp)
    }
}

impl<T, I, F> Iterator for MergeSortIterator<T, I, F>
where
    T: Clone + Debug,
    I: Iterator<Item = T>,
    F: Fn(&T, &T) -> Ordering,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.buffered.is_empty() {
            let idx = self.arg_cmp_by();
            let ret = self.swap_next(idx);
            #[cfg(debug_assertions)]
            self.continuation_check(idx, &ret);
            Some(ret)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_sort_basic() {
        let i1 = vec![1u32, 6, 15];
        let i2 = vec![4u32, 9, 12];
        let i3 = vec![2u32, 8, 11];
        let i4 = vec![5u32, 7, 14];
        let i5 = vec![3u32, 10, 13];

        let iter = MergeSortIterator::new(
            vec![
                i1.into_iter(),
                i2.into_iter(),
                i3.into_iter(),
                i4.into_iter(),
                i5.into_iter(),
            ],
            Order::Asc,
        );
        assert_eq!(
            iter.collect::<Vec<u32>>(),
            vec![1u32, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]
        );
    }

    #[test]
    fn test_merge_sort() {
        let mut vectors = Vec::with_capacity(1000);

        for _ in 0..199 {
            vectors.push(vec![]);
        }
        vectors.push(vec![1u32]);
        for _ in 200..399 {
            vectors.push(vec![]);
        }
        vectors.push(vec![4u32, 9, 12]);
        for _ in 400..599 {
            vectors.push(vec![]);
        }
        vectors.push(vec![2u32, 8, 11, 14]);
        for _ in 600..799 {
            vectors.push(vec![]);
        }
        vectors.push(vec![5u32, 7]);
        for _ in 800..999 {
            vectors.push(vec![]);
        }
        vectors.push(vec![3u32, 6, 10, 13, 15]);

        let iter = MergeSortIterator::new(
            vectors.into_iter().map(|v| v.into_iter()).collect(),
            Order::Asc,
        );
        assert_eq!(
            iter.collect::<Vec<u32>>(),
            vec![1u32, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]
        );
    }
}
