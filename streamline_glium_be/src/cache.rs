/// A cache  to reduce memory transfers.
/// instead of coping the vertices buffers all the time
/// we will detect reusage and store vb for the next frame

use lru::LruCache;
use glium::VertexBuffer;
use std::hash::Hasher;
use seahash::SeaHasher;

use streamline::tools::LayoutHash;

pub struct VbCache<T>
{
    cache: LruCache< u64, T>,
}

#[cfg_attr(feature="profile", flame)]
fn do_hash<L> (layout: &[L]) -> u64
where L: LayoutHash{
    let mut h = SeaHasher::new();
    for i in layout{
        i.hash(&mut h);
    }
    h.finish()
}

impl<T> VbCache<T>
{
    pub fn new() -> VbCache<T>{
        VbCache{
            cache: LruCache::new(128)
        }
    }

    /// tests whenever an input is cached already. 
    /// If not, a closure generating the value is given
    #[cfg_attr(feature="profile", flame)]
    pub fn test<F, L> (&mut self, layout: &[L], mut f: F) -> &T
        where L: LayoutHash,
              F: FnMut()->T
    {

        let hash = do_hash(layout);

        if !self.cache.contains(&hash){
            // cache  the thing
            self.cache.put(hash,f());
        }

        self.cache.get(&hash).expect("we just added it")
    }
}
