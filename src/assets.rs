use SpriteId;
use FontId;
use StreamLineBackend;

use std::path::PathBuf;
use image;
use image::DynamicImage;
use image::GenericImage;
use image::RgbaImage;

use std::vec::Vec;
use std::hash::Hash;
use std::hash::Hasher;
use std::collections::hash_map::DefaultHasher;
use std::iter::FromIterator;

use std::{fs, io};

use rect_packer;

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

fn gen_atlas(dim: (f32, f32),
             layout: &Map<SpriteId, rect_packer::Rect>,
             images: &[DynamicImage])
             -> RgbaImage {
    println!("new atlas:{:?}", dim);
    let mut atlas_image = image::RgbaImage::new(dim.0 as u32, dim.1 as u32);

    for (i, img) in images.iter().enumerate() {
        let rect = layout[&i];
        println!("copy:{} {:?}", i, rect);
        let mut cell = image::SubImage::new(&mut atlas_image,
                                            rect.x as u32,
                                            rect.y as u32,
                                            rect.width as u32,
                                            rect.height as u32);
        assert!(cell.copy_from(img, 0, 0));
    }

    atlas_image
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

#[derive(Debug, Copy, Clone)]
pub enum AssetsMgrError {
    LoadError,
}

#[derive(Debug, Copy, Clone)]
pub struct Rect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}
use std::collections::BTreeMap as Map;

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

/// the assets manager builder is used to collect all the assets
/// when calling the build method, a read only asset manager object is
/// generated, all meta data is backed up buy the right backend structure
pub struct AssetsMgrBuilder<'a, BE>
    where BE: StreamLineBackend + 'a
{
    be: &'a mut BE,
    sprites_to_include: Vec<PathBuf>,
    fonts_to_include: Vec<u32>,
}

impl<'a, BE> AssetsMgrBuilder<'a, BE>
    where BE: StreamLineBackend + 'a
{
    /// create an assets manager builder, to be filled with assets and then
    /// locked for use
    pub fn new(be: &'a mut BE) -> AssetsMgrBuilder<'a, BE> {
        AssetsMgrBuilder {
            be: be,
            sprites_to_include: Vec::new(),
            fonts_to_include: Vec::new(),
        }
    }

    /// adds one file into the assets set
    pub fn add_sprite(&mut self, path: &PathBuf) -> SpriteId {
        let id = self.sprites_to_include.len();
        self.sprites_to_include.push(path.clone());
        id
    }

    pub fn add_font(&mut self, path: &PathBuf) -> Result<FontId, io::Error> {
        let i = self.fonts_to_include.len();
        let font_file = fs::File::open(path)?;
        self.fonts_to_include.push(self.be.add_font(font_file));
        Ok(i)
    }

    /// creates the assets manager object, with all the submitted images
    pub fn build(self) -> Result<AssetsMgr, AssetsMgrError> {

        // TODO: cache the results
        let _hash = calculate_hash(&self.sprites_to_include);

        let iter = self.sprites_to_include.iter();
        // load all images
        let images: Vec<DynamicImage> = iter.map(|path| image::open(path.as_path()))
            .take_while(Result::is_ok)
            .map(Result::unwrap)
            .collect();

        if images.len() != self.sprites_to_include.len() {
            return Err(AssetsMgrError::LoadError);
        }


        let (width, height) = (1024, 1024);
        let config = rect_packer::Config {
            width: 1024,
            height: 1024,

            border_padding: 5,
            rectangle_padding: 10,
        };
        let dim = (width as f32, height as f32);

        let mut packer = rect_packer::Packer::new(config);
        let mut sprites_loc_map = Map::new();
        let mut int_map = Map::new();
        for (i, img) in images.iter().enumerate() {
            let (w, h) = img.dimensions();
            let frame = packer.pack(w as i32, h as i32, false)
                .expect("textures do not fit in map");

            sprites_loc_map.insert(i,
                                   Rect {
                                       x: frame.x as f32 / dim.0,
                                       y: 1.0 - frame.y as f32 / dim.1,
                                       w: frame.width as f32 / dim.0,
                                       h: frame.height as f32 / dim.1,
                                   });
            int_map.insert(i, frame);
        }

        let atlas = gen_atlas(dim, &int_map, &images);

        // Write the contents of this image to the Writer in PNG format.
        // TODO: cache it whith the hash of the names list
        // atlas.save("atlas.png").unwrap();

        // load atlas image on backend
        let tex = self.be.add_texture(atlas);

        // now the fonts
        let font_map = Map::from_iter(self.fonts_to_include
            .iter()
            .enumerate()
            .map(|kv| (kv.0, *kv.1)));

        Ok(AssetsMgr {
            _total_size: dim,
            sprite_locations: sprites_loc_map,
            fonts: font_map,
            tex: tex,
        })
    }
}

/// Describes every texture previously registered
/// All meta data is backed up buy the right backend structure
pub struct AssetsMgr {
    _total_size: (f32, f32),
    sprite_locations: Map<SpriteId, Rect>,
    fonts: Map<FontId, u32>,
    tex: u32,
}

impl AssetsMgr {
    /// get location in the atlas for a given sprite
    pub fn get_sprite_offset(&self, id: SpriteId) -> Option<(f32, f32)> {
        if let Some(rect) = self.sprite_locations.get(&id) {
            return Some((rect.x, rect.y));
        }
        None
    }
    /// get size of a sprite
    pub fn get_sprite_size(&self, id: SpriteId) -> Option<(f32, f32)> {
        if let Some(rect) = self.sprite_locations.get(&id) {
            return Some((rect.w, rect.h));
        }
        None
    }
    /// returns the atlas texture identifier as regisitered in the backend
    pub fn get_atlas(&self) -> u32 {
        self.tex
    }

    pub fn get_font(&self, id: &FontId) -> u32 {
        return self.fonts[id];
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use super::StreamLineBackend;
    use image::RgbaImage;

    use std::io;

    struct TestBE;
    impl StreamLineBackend for TestBE {
        type Surface = TestBESurface;
        fn add_texture(&mut self, _img: RgbaImage) -> u32 {
            0
        }
        fn add_font<FIO: io::Read>(&mut self, _font: FIO) -> u32 {
            0
        }
        fn surface(&mut self, _: u32) -> Self::Surface {
            TestBESurface {}
        }
    }
    struct TestBESurface;

    #[test]
    fn load_sprites() {

        use find_folder::Search;
        use std::path::Path;

        let mut file_location = Search::Parents(3)
            .for_folder("assets")
            .expect("some assets folder must exist somewhere");
        file_location.push(Path::new("rust-logo.png"));

        let mut be = TestBE {};

        let mgr = {
            let mut builder = AssetsMgrBuilder::new(&mut be);
            builder.add_sprite(&file_location);
            builder.add_sprite(&file_location);
            builder.add_sprite(&file_location);
            builder.add_sprite(&file_location);
            builder.add_sprite(&file_location);
            builder.build().expect("the assets must be found")
        };

        println!("size 0: {:?}", mgr.get_sprite_size(0).unwrap());
        println!("size 1: {:?}", mgr.get_sprite_size(1).unwrap());
        println!("size 2: {:?}", mgr.get_sprite_size(2).unwrap());
        println!("size 3: {:?}", mgr.get_sprite_size(3).unwrap());
        println!("size 4: {:?}", mgr.get_sprite_size(4).unwrap());
        assert!(mgr.get_sprite_size(5).is_none());

        println!("offset 0: {:?}", mgr.get_sprite_offset(0).unwrap());
        println!("offset 1: {:?}", mgr.get_sprite_offset(1).unwrap());
        println!("offset 2: {:?}", mgr.get_sprite_offset(2).unwrap());
        println!("offset 3: {:?}", mgr.get_sprite_offset(3).unwrap());
        println!("offset 4: {:?}", mgr.get_sprite_offset(4).unwrap());
        assert!(mgr.get_sprite_offset(5).is_none());
    }

    #[test]
    fn load_font() {

        use find_folder::Search;
        use std::path::Path;

        let mut file_location = Search::Parents(3)
            .for_folder("assets")
            .expect("some assets folder must exist somewhere");
        file_location.push(Path::new("OpenSans-Regular.ttf"));

        let mut be = TestBE {};

        let mgr = {
            let mut builder = AssetsMgrBuilder::new(&mut be);
            assert!(builder.add_font(&file_location).is_ok());
            file_location.push(Path::new("nop"));
            assert!(builder.add_font(&file_location).is_err());
            builder.build().expect("nothin fancy anymore")
        };

        assert_eq!(mgr.get_font(&0), 0);
    }
}
