from __future__ import annotations
from dataclasses import dataclass
from typing import Iterable

LONGITUDES=("west","center","east")
ALTITUDES=("low","medium","high")

@dataclass(frozen=True)
class BiomeKey:
    continent:str; longitude:str; altitude:str
    @property
    def id(self): return f"{self.continent}_{self.longitude}_{self.altitude}"

@dataclass(frozen=True)
class ProceduralAssetSpec:
    id:str; category:str; weight:float; radius_m:float; scale_min:float; scale_max:float
    width_m:float; height_m:float; depth_m:float=0.0; planes:int=0; variant_index:int=0

def normalize_biome(continent,longitude,altitude):
    c=str(continent).lower(); lon=str(longitude).lower(); alt=str(altitude).lower()
    if c!="south_america": raise KeyError("Only south_america is currently supported")
    if lon not in LONGITUDES: raise KeyError(f"longitude must be one of {LONGITUDES}")
    if alt not in ALTITUDES: raise KeyError(f"altitude must be one of {ALTITUDES}")
    return BiomeKey(c,lon,alt)

def biome_from_config(config):
    env=config["procedural_environment"]; b=env.get("biome",{})
    return normalize_biome(b.get("continent",env.get("region","south_america")),b.get("longitude","west"),b.get("altitude","low"))

def supported_biomes(): return tuple(BiomeKey("south_america",lon,alt) for lon in LONGITUDES for alt in ALTITUDES)

def _shift(c,green=0.0,dry=0.0,dark=0.0):
    r,g,b=c; return (max(0,min(1,r+dry-dark)),max(0,min(1,g+green-dark)),max(0,min(1,b+dry*0.35-dark*0.55)))

def palette_for(b):
    wet={"west":-0.035,"center":0.015,"east":0.075}[b.longitude]
    cold={"low":0.0,"medium":0.015,"high":0.035}[b.altitude]
    green=_shift((0.29,0.37,0.18),green=wet-cold,dark=cold*0.35)
    dry=_shift((0.46,0.42,0.23),green=wet*0.25,dry=-wet*0.25,dark=cold*0.20)
    dirt=_shift((0.43,0.31,0.20),green=wet*0.08,dark=cold*0.10)
    mix={"west":(0.40,0.45,0.15),"center":(0.55,0.32,0.13),"east":(0.70,0.20,0.10)}[b.longitude]
    if b.altitude=="high": mix=(mix[0]*0.86,mix[1]*1.16,mix[2])
    def variants(base,contrast):
        return tuple(_shift(base,green=(i-1.5)*contrast*0.18,dry=(1.5-i)*contrast*0.13,dark=(i%2)*contrast*0.10) for i in range(4))
    return {
      "terrain":{"green":green,"dry":dry,"dirt":dirt,"mix":mix},
      "tree":variants(_shift(green,green=-0.01,dark=0.035),0.12),
      "bush":variants(_shift(green,green=0.015,dry=0.045),0.14),
      "grass":variants(_shift(dry,green=0.01),0.16),
      "structures":variants((0.56,0.53,0.47),0.11),
    }

def _profile(b):
    moisture={"west":0.94,"center":1.0,"east":1.08}[b.longitude]
    height_factor={"low":1.03,"medium":1.0,"high":0.90}[b.altitude]
    prefix=b.id
    tree_dims=((6.2,9.2),(7.0,11.0),(5.5,13.0),(7.8,10.2))
    bush_dims=((3.2,1.55),(3.8,1.75),(2.8,1.35),(4.2,1.65))
    grass_dims=((0.62,0.55),(0.78,0.62),(0.54,0.48),(0.70,0.68))
    build_dims=((10,5.5,6.5),(18,7,9),(8,7.5,5.5),(13,6.2,7.5))
    bw={"west":1.0,"center":0.96,"east":1.08}[b.longitude]; bh={"low":0.96,"medium":1.0,"high":1.10}[b.altitude]; bd={"west":1.0,"center":0.96,"east":1.06}[b.longitude]
    trees=tuple(ProceduralAssetSpec(f"{prefix}_tree_{i+1:02d}","trees",1, w*.48*moisture,.88,1.18,w*moisture,h*height_factor*moisture,planes=3,variant_index=i) for i,(w,h) in enumerate(tree_dims))
    bushes=tuple(ProceduralAssetSpec(f"{prefix}_bush_{i+1:02d}","bushes",1,w*.46,.84,1.20,w,h*(.96+.08*moisture),planes=2,variant_index=i) for i,(w,h) in enumerate(bush_dims))
    grass=tuple(ProceduralAssetSpec(f"{prefix}_grass_{i+1:02d}","grass",1,w*.35,.78,1.22,w,h,planes=1,variant_index=i) for i,(w,h) in enumerate(grass_dims))
    buildings=tuple(ProceduralAssetSpec(f"{prefix}_building_{i+1:02d}","fake_buildings",1,max(w*bw,d*bd)*.60,.90,1.10,w*bw,h*bh,d*bd,variant_index=i) for i,(w,h,d) in enumerate(build_dims))
    return {"trees":trees,"bushes":bushes,"grass":grass,"fake_buildings":buildings}

def specs_for_biome(b,category): return _profile(b)[category]
def spec_map_for_biome(b): return {s.id:s for values in _profile(b).values() for s in values}
def weighted_choice(rng,specs:Iterable[ProceduralAssetSpec]):
    vals=tuple(specs); total=sum(max(0,s.weight) for s in vals); pick=rng.random()*total if total else 0; run=0
    for s in vals:
        run+=max(0,s.weight)
        if pick<=run:return s
    return vals[-1]
