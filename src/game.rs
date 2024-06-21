use crate::structures::{CommodityGroup, Structure, StructureGroup};
use std::collections::hash_map::{Iter, IterMut};
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter, Result};
use worldgen::noise::perlin::PerlinNoise;
use worldgen::noisemap::{NoiseMap, NoiseMapGenerator, Seed, Step};
use worldgen::world::tile::Constraint;
use worldgen::world::tile::ConstraintType;
use worldgen::world::{Size, Tile, World};

type WorldCache = Vec<Vec<MapTile>>;

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct Position {
    pub x: i16,
    pub y: i16,
}

impl Position {
    pub fn new(x: i16, y: i16) -> Position {
        return Position { x, y };
    }

    pub fn x(&mut self, x: i16) {
        self.x = x
    }

    pub fn y(&mut self, y: i16) {
        self.y = y
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ResourceGroup {
    Energy,
    Metal,
    Mineral,
    Gas,
    Carbon,
}

impl Display for ResourceGroup {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Clone)]
pub struct Resource {
    group: ResourceGroup,
    amount: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Flora {
    Water,
    Sand,
    Dirt,
    Grass,
    Rock,
}

impl Display for Flora {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{:?}", self)
    }
}

pub struct ResourceManager {
    resources: HashMap<ResourceGroup, u64>,
    commodities: HashMap<CommodityGroup, u64>,
}

impl ResourceManager {
    pub fn new(
        resource_types: Vec<ResourceGroup>,
        commodity_types: Vec<CommodityGroup>,
    ) -> ResourceManager {
        let mut resources: HashMap<ResourceGroup, u64> = HashMap::new();
        for resource_type in resource_types {
            resources.insert(resource_type, 0);
        }

        let mut commodities: HashMap<CommodityGroup, u64> = HashMap::new();
        for commodity_type in commodity_types {
            commodities.insert(commodity_type, 0);
        }

        return ResourceManager {
            resources,
            commodities,
        };
    }

    pub fn list_resources(&self) -> Iter<'_, ResourceGroup, u64> {
        return self.resources.iter();
    }

    pub fn list_resources_mut(&mut self) -> IterMut<'_, ResourceGroup, u64> {
        return self.resources.iter_mut();
    }

    pub fn deposit_resource(&mut self, resource_type: &ResourceGroup, value: u64) -> bool {
        if value == 0 {
            return false;
        }
        let amount = self.resources.get_mut(&resource_type).unwrap();
        *amount += value;
        return true;
    }

    pub fn withdraw_resource(&mut self, resource_type: &ResourceGroup, value: u64) -> bool {
        let amount = self.resources.get_mut(&resource_type).unwrap();
        if *amount < value {
            return false;
        }
        *amount -= value;
        return true;
    }

    pub fn list_commodities(&self) -> Iter<'_, CommodityGroup, u64> {
        return self.commodities.iter();
    }

    pub fn list_commodities_mut(&mut self) -> IterMut<'_, CommodityGroup, u64> {
        return self.commodities.iter_mut();
    }

    pub fn deposit_commodity(&mut self, resource_type: &CommodityGroup, value: u64) -> bool {
        if value == 0 {
            return false;
        }
        let amount = self.commodities.get_mut(&resource_type).unwrap();
        *amount += value;
        return true;
    }

    pub fn withdraw_commodity(&mut self, resource_type: &CommodityGroup, value: u64) -> bool {
        let amount = self.commodities.get_mut(&resource_type).unwrap();
        if *amount < value {
            return false;
        }
        *amount -= value;
        return true;
    }
}

#[derive(Clone)]
pub struct MapTile {
    pub flora: Flora,
}

pub struct MapObject {
    pub resource: Option<Resource>,
    pub structure: Option<Structure>,
}

pub struct ObjectManager {
    objects: HashMap<Position, MapObject>,
}

impl ObjectManager {
    pub fn new() -> ObjectManager {
        let objects = HashMap::new();
        ObjectManager { objects }
    }
    pub fn get(&self, position: &Position) -> Option<&MapObject> {
        self.objects.get(position)
    }

    pub fn get_mut(&mut self, position: &Position) -> Option<&mut MapObject> {
        self.objects.get_mut(position)
    }

    pub fn set(&mut self, position: Position, object: MapObject) -> Option<MapObject> {
        self.objects.insert(position, object)
    }

    pub fn remove(&mut self, position: &Position) -> Option<MapObject> {
        self.objects.remove(&position)
    }

    pub fn list(&self) -> Iter<'_, Position, MapObject> {
        return self.objects.iter();
    }

    pub fn list_mut(&mut self) -> IterMut<'_, Position, MapObject> {
        return self.objects.iter_mut();
    }
}

pub struct MapTileFactory {}

impl MapTileFactory {
    pub fn new(flora: Flora) -> MapTile {
        return MapTile { flora };
    }
}

pub struct GameMap {
    width: u16,
    height: u16,
    world: World<MapTile>,
    cache: Option<WorldCache>,
}

impl GameMap {
    pub fn new(width: u16, height: u16) -> GameMap {
        let noise = PerlinNoise::new();

        let nm1 = NoiseMap::new(noise)
            .set_seed(Seed::of("FooMoo!"))
            .set_step(Step::of(0.005, 0.005));

        let nm2 = NoiseMap::new(noise)
            .set_seed(Seed::of("!GooToo"))
            .set_step(Step::of(0.05, 0.05));

        let nm = Box::new(nm1 + nm2 * 4);

        let water_tile = MapTileFactory::new(Flora::Water);
        let sand_tile = MapTileFactory::new(Flora::Sand);
        let grass_tile = MapTileFactory::new(Flora::Grass);
        let dirt_tile = MapTileFactory::new(Flora::Dirt);
        let rock_tile = MapTileFactory::new(Flora::Rock);

        let world = World::new()
            .set(Size::of(width as i64, height as i64))
            // Water
            .add(Tile::new(water_tile).when(constraint!(nm.clone(), < -0.25)))
            // Sand
            .add(Tile::new(sand_tile).when(constraint!(nm.clone(), < 0.0)))
            // Grass
            .add(Tile::new(grass_tile).when(constraint!(nm.clone(), < 0.45)))
            // Mountains
            .add(Tile::new(rock_tile).when(constraint!(nm.clone(), > 0.8)))
            // Hills
            .add(Tile::new(dirt_tile));

        let cache = world.generate(0, 0);

        return GameMap {
            width,
            height,
            world,
            cache,
        };
    }

    pub fn world(&self) -> &World<MapTile> {
        return &self.world;
    }

    pub fn cache(&self) -> &WorldCache {
        return self.cache.as_ref().unwrap();
    }

    pub fn width(&self) -> u16 {
        return self.width;
    }

    pub fn height(&self) -> u16 {
        return self.height;
    }
}

pub struct MapController {
    map: GameMap,
    objects: ObjectManager,
    position: Position,
    locations: HashMap<Position, StructureGroup>,
}

impl MapController {
    pub fn new(size: Size) -> MapController {
        let position = Position::new(0, 0);
        let (w, h) = (size.w as u16, size.h as u16);

        let map = GameMap::new(w, h);

        let objects = ObjectManager::new();

        let locations = HashMap::new();

        return MapController {
            map,
            objects,
            position,
            locations,
        };
    }

    pub fn map(&self) -> &GameMap {
        return &self.map;
    }

    pub fn locations(&self) -> &HashMap<Position, StructureGroup> {
        return &self.locations;
    }

    pub fn objects(&self) -> &ObjectManager {
        return &self.objects;
    }

    pub fn objects_mut(&mut self) -> &mut ObjectManager {
        return &mut self.objects;
    }

    pub fn add_object(&mut self, position: Position, object: MapObject) -> Option<MapObject> {
        self.objects.set(position, object)
    }

    pub fn remove_object(&mut self, position: &Position) -> Option<MapObject> {
        self.objects.remove(position)
    }

    pub fn add_structure(&mut self, structure: Structure) {
        let position = self.position().clone();

        let object = MapObject {
            structure: Option::from(structure),
            resource: Option::None,
        };

        self.add_object(position, object);
    }

    pub fn position(&self) -> Position {
        return self.position.clone();
    }

    pub fn tile(&self) -> &MapTile {
        let x = self.position.x as usize;
        let y = self.position.y as usize;

        return &self.map.cache.as_ref().unwrap()[y][x];
    }

    pub fn tile_at(&self, position: Position) -> &MapTile {
        let x = position.x as usize;
        let y = position.y as usize;

        return &self.map.cache.as_ref().unwrap()[y][x];
    }

    pub fn tile_at_mut(&mut self, position: Position) -> &mut MapTile {
        let x = position.x as usize;
        let y = position.y as usize;

        return &mut self.map.cache.as_mut().unwrap()[y][x];
    }

    pub fn object(&self) -> Option<&MapObject> {
        return self.objects.get(&self.position);
    }

    pub fn up(&mut self) {
        let y = self.position.y as u16;
        if y > 0 {
            self.position.y((y - 1) as i16);
        }
    }

    pub fn down(&mut self) {
        let y = self.position.y as u16;
        if y < self.map.height() - 1 {
            self.position.y((y + 1) as i16);
        }
    }

    pub fn right(&mut self) {
        let x = self.position.x as u16;
        if x < self.map.width() - 1 {
            self.position.x((x + 1) as i16);
        }
    }

    pub fn left(&mut self) {
        let x = self.position.x as u16;
        if x > 0 {
            self.position.x((x - 1) as i16);
        }
    }
}
