use sequence_trie::SequenceTrie;

pub struct Trie<V> {
    trie: SequenceTrie<char, Vec<V>>,
}
impl <V> Trie<V> {
    pub fn new() -> Self {
        Self {
            trie: SequenceTrie::new(),
        }
    }
    pub fn insert(&mut self, k: &[char], v: V) {
        let n = k.len();
        for i in 1..(n+1) {
            let exists = self.trie.get_node(&k[0..i]).is_some();
            if !exists {
                self.trie.insert(&k[0..i], vec![]);
            };
        }
        let vv: &mut Vec<V> = self.trie.get_mut(k).unwrap();
        vv.push(v)
    }
    pub fn get_node(&self, k: &[char]) -> Option<TrieView<V>> {
        let mut s = String::new();
        for &c in k {
            s.push(c);
        }
        self.trie.get_node(k).map(|tr| TrieView { searched_key:s, trie: tr })
    }
}

pub struct TrieView<'a, V> {
    searched_key: String,
    trie: &'a SequenceTrie<char, Vec<V>>
}
impl <'a, V: Clone> TrieView<'a, V> {
    pub fn list_values(&self) -> Vec<(String, Vec<V>)> {
        let mut vvv = vec![];
        for (k, vv) in self.trie.iter() {
            let mut s = self.searched_key.clone();
            for &c in k {
                s.push(c);
            }
            if !vv.is_empty() {
                vvv.push((s, vv.to_vec()));
            }
        }
        vvv.sort_by_key(|s| s.0.len());
        vvv
    }
}

#[test]
fn test_trie() {
    let mut tr = Trie::new();
    tr.insert(&['a','b','c'], 1);
    tr.insert(&['a','b'], 2);
    tr.insert(&['a','b','c'], 3);
    assert!(tr.get_node(&['a','c']).is_none());
    assert_eq!(tr.get_node(&['a']).unwrap().list_values(), vec![("ab".to_owned(), vec![2]),("abc".to_owned(),vec![1,3])]);
    assert_eq!(tr.get_node(&['a','b']).unwrap().list_values(), vec![("ab".to_owned(), vec![2]),("abc".to_owned(),vec![1,3])]);
}