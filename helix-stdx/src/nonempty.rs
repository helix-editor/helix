use serde::{de::Error, ser::SerializeSeq, Deserialize, Deserializer, Serialize, Serializer};

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct EmptyError;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct NonEmptyVec<T>
where
    T: Clone,
{
    head: T,
    tail: Vec<T>,
}

impl<T: Clone> NonEmptyVec<T> {
    pub fn head(&self) -> &T {
        &self.head
    }

    pub fn tail(&self) -> &Vec<T> {
        &self.tail
    }

    pub fn from_vec(mut vec: Vec<T>) -> Option<Self> {
        if vec.is_empty() {
            None
        } else {
            let head = vec.remove(0);
            Some(Self { head, tail: vec })
        }
    }
}

impl<T: Serialize + Clone> Serialize for NonEmptyVec<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.tail.len() + 1))?;
        seq.serialize_element(&self.head)?;
        for e in &self.tail {
            seq.serialize_element(&e)?;
        }
        seq.end()
    }
}

impl<'de, T: Deserialize<'de> + Clone> Deserialize<'de> for NonEmptyVec<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let v = <Vec<T>>::deserialize(deserializer)?;
        let mut v = v.into_iter();

        if let Some(first) = v.next() {
            Ok(NonEmptyVec {
                head: first,
                tail: v.collect(),
            })
        } else {
            Err(D::Error::custom("vector must be non-empty"))
        }
    }
}
