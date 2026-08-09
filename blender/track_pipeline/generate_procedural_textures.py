from __future__ import annotations

import argparse
from pathlib import Path
import hashlib
import json
import math

import numpy as np
from PIL import Image, ImageDraw, ImageFilter

from pipeline_common import read_json
from procedural_catalog import biome_from_config, palette_for, supported_biomes, specs_for_biome


def seed_for(base_seed: int, name: str) -> int:
    digest=hashlib.sha256(f"{base_seed}:{name}".encode("utf-8")).digest()
    return int.from_bytes(digest[:8],"little",signed=False)

def rng_for(base_seed: int,name: str): return np.random.default_rng(seed_for(base_seed,name))
def rgb8(color): return tuple(int(max(0,min(255,round(float(c)*255)))) for c in color)
def vary(color,factor):
    c=np.clip(np.asarray(color,dtype=float)*factor,0,1); return (*rgb8(c),255)

def low_frequency_field(rng,size,coarse=9):
    small=rng.random((coarse,coarse)).astype(np.float32)
    img=Image.fromarray((small*255).astype(np.uint8),"L").resize((size,size),Image.Resampling.BICUBIC).filter(ImageFilter.GaussianBlur(radius=max(1.0,size/48)))
    return np.asarray(img,dtype=np.float32)/255.0

def save_asphalt(path,size,seed):
    rng=rng_for(seed,"asphalt"); fine=rng.normal(0,0.017,(size,size)).astype(np.float32); macro=low_frequency_field(rng,size,11)-0.5; warm=low_frequency_field(rng,size,7)-0.5
    value=np.clip(0.185+fine+macro*0.050,0.105,0.285)
    rgb=np.stack([value*(0.985+warm*0.025),value,value*(1.015-warm*0.020)],axis=-1)
    Image.fromarray(np.clip(rgb*255,0,255).astype(np.uint8),"RGB").save(path)

def save_terrain(path,size,seed,palette):
    rng=rng_for(seed,path.as_posix()); green=np.asarray(palette["terrain"]["green"],np.float32); dry=np.asarray(palette["terrain"]["dry"],np.float32); dirt=np.asarray(palette["terrain"]["dirt"],np.float32); gm,dm,tm=map(float,palette["terrain"]["mix"])
    f1=low_frequency_field(rng,size,8); f2=low_frequency_field(rng,size,6); f3=low_frequency_field(rng,size,5)
    yy,xx=np.mgrid[0:size,0:size].astype(np.float32)
    gx=(xx-size*0.30)/(size*0.45); gy=(yy-size*0.63)/(size*0.58); green_bias=np.exp(-(gx*gx+gy*gy)*1.35)
    dx=(xx-size*0.76)/(size*0.28); dy=(yy-size*0.27)/(size*0.34); dirt_bias=np.exp(-(dx*dx+dy*dy)*1.75)
    base=green*gm+dry*dm+dirt*tm
    green_patch=np.clip((f1-0.46)*1.25+green_bias*0.62,0,1); dry_patch=np.clip((f2-0.54)*1.10,0,0.72); dirt_patch=np.clip((f3-0.64)*1.55+dirt_bias*0.70,0,0.78)
    rgb=np.broadcast_to(base,(size,size,3)).copy(); rgb+=(green-base)[None,None,:]*green_patch[...,None]*0.42; rgb+=(dry-base)[None,None,:]*dry_patch[...,None]*0.30; rgb+=(dirt-base)[None,None,:]*dirt_patch[...,None]*0.38
    rgb=np.clip(rgb+rng.normal(0,0.013,(size,size,1)).astype(np.float32),0,1)
    Image.fromarray((rgb*255).astype(np.uint8),"RGB").save(path)

def save_tree_card(path,size,seed,base_color,variant):
    rng=rng_for(seed,path.as_posix()); image=Image.new("RGBA",(size,size),(0,0,0,0)); draw=ImageDraw.Draw(image)
    trunk_w=size*(0.055+0.012*(variant%2)); trunk_top=size*(0.38+0.05*(variant==2))
    draw.polygon([(size/2-trunk_w,size*0.94),(size/2+trunk_w,size*0.94),(size/2+trunk_w*0.65,trunk_top),(size/2-trunk_w*0.55,trunk_top)],fill=(91,65,39,255))
    centers={0:[(.50,.40,.29,.22),(.34,.48,.22,.18),(.66,.47,.22,.18),(.50,.28,.20,.15)],1:[(.50,.38,.33,.20),(.28,.46,.20,.16),(.72,.46,.20,.16),(.50,.24,.16,.12)],2:[(.50,.32,.20,.28),(.43,.48,.17,.22),(.60,.46,.16,.21),(.50,.18,.13,.15)],3:[(.48,.42,.34,.22),(.25,.47,.18,.14),(.72,.48,.24,.17),(.58,.27,.19,.14)]}[variant]
    for idx,(cx,cy,rx,ry) in enumerate(centers):
        jitter=rng.uniform(.94,1.07); box=((cx-rx*jitter)*size,(cy-ry*jitter)*size,(cx+rx*jitter)*size,(cy+ry*jitter)*size); factor=[.86,1,1.10,.94][idx%4]
        draw.ellipse(box,fill=vary(base_color,factor)); x0,y0,x1,y1=box
        draw.polygon([(x0+(x1-x0)*.18,y0+(y1-y0)*.18),(x0+(x1-x0)*.55,y0+(y1-y0)*.08),(x0+(x1-x0)*.44,y0+(y1-y0)*.42)],fill=vary(base_color,min(1.22,factor+.14)))
    draw.polygon([(size*.53,size*.47),(size*.78,size*.43),(size*.69,size*.60),(size*.48,size*.61)],fill=vary(base_color,.76)); image.save(path)

def save_bush_card(path,size,seed,base_color,variant):
    rng=rng_for(seed,path.as_posix()); image=Image.new("RGBA",(size,size),(0,0,0,0)); draw=ImageDraw.Draw(image); draw.rectangle((size*.47,size*.72,size*.53,size*.96),fill=(86,65,41,230))
    for _ in range(13+variant*2):
        rx=rng.uniform(size*.08,size*.16); ry=rng.uniform(size*.07,size*.14); cx=rng.uniform(size*.18,size*.82); cy=rng.uniform(size*.43,size*.79); lighting=.83+.30*(1-(cx/size)*.65-(cy/size)*.20)
        draw.ellipse((cx-rx,cy-ry,cx+rx,cy+ry),fill=vary(base_color,lighting))
    draw.polygon([(size*.12,size*.76),(size*.24,size*.67),(size*.43,size*.72),(size*.58,size*.66),(size*.76,size*.71),(size*.88,size*.78),(size*.80,size*.84),(size*.18,size*.84)],fill=vary(base_color,.82)); image.save(path)

def save_grass_card(path,size,seed,base_color,variant):
    rng=rng_for(seed,path.as_posix()); image=Image.new("RGBA",(size,size),(0,0,0,0)); draw=ImageDraw.Draw(image)
    for _ in range(14+variant*3):
        x0=size*.5+rng.uniform(-size*.28,size*.28); y0=size*.96; length=rng.uniform(size*.34,size*(.70+.04*variant)); lean=rng.uniform(-size*.23,size*.23)
        draw.line((x0,y0,x0+lean,y0-length),fill=vary(base_color,rng.uniform(.78,1.16)),width=max(1,int(size/64)))
    image.save(path)

def save_building_facade(path,size,seed,base_color,variant,biome):
    rng=rng_for(seed,path.as_posix()); image=Image.new("RGBA",(size,size),(*rgb8(base_color),255)); draw=ImageDraw.Draw(image); industrial=variant in (1,3); rows=2 if industrial else 4; cols=(5+variant) if industrial else (4+variant%2); margin=size*.08; bottom=size*.84; cell_w=(size-2*margin)/(cols*1.32); xgap=(size-2*margin-cols*cell_w)/max(1,cols-1); cell_h=(bottom-margin)/(rows*1.55); ygap=(bottom-margin-rows*cell_h)/max(1,rows)
    cornice=.075 if biome.altitude=="high" else (.040 if biome.longitude=="east" else .055); cf=.63 if biome.altitude=="high" else (.82 if biome.longitude=="east" else .68); draw.rectangle((0,0,size,size*cornice),fill=vary(base_color,cf))
    for r in range(rows):
        for c in range(cols):
            x0=margin+c*(cell_w+xgap); y0=margin+r*(cell_h+ygap); window=(48,58,61,255) if not industrial else (58,65,65,255)
            if rng.random()<.06: window=(169,153,105,255)
            draw.rectangle((x0-2,y0-2,x0+cell_w+2,y0+cell_h+2),fill=(35,35,33,110)); draw.rectangle((x0,y0,x0+cell_w,y0+cell_h),fill=window)
    if biome.altitude=="high" and not industrial: draw.rectangle((0,size*.82,size,size*.90),fill=vary(base_color,.78))
    elif biome.longitude=="east" and not industrial: draw.rectangle((0,size*.80,size,size*.87),fill=vary(base_color,1.12))
    if industrial:
        door_w=size*(.22 if variant==1 else .28); draw.rectangle((size*.5-door_w/2,size*.60,size*.5+door_w/2,size*.96),fill=(82,79,72,255))
        for y in np.linspace(size*.64,size*.91,4): draw.line((size*.5-door_w/2,y,size*.5+door_w/2,y),fill=(52,52,49,255),width=1)
    else: draw.rectangle((size*.44,size*.67,size*.56,size*.96),fill=(75,66,56,255))
    draw.rectangle((0,size*.90,size,size),fill=vary(base_color,.72)); image.save(path)

def save_simple_noise(path,size,base_rgb,strength,seed):
    rng=rng_for(seed,path.as_posix()); base=np.asarray(base_rgb,np.float32); macro=low_frequency_field(rng,size,8)-.5; fine=rng.normal(0,strength,(size,size,1)).astype(np.float32); rgb=np.clip(base[None,None,:]+macro[...,None]*strength*1.3+fine,0,1); Image.fromarray((rgb*255).astype(np.uint8),"RGB").save(path)

def save_checker(path,size):
    image=Image.new("RGB",(size,size),(238,238,238)); draw=ImageDraw.Draw(image); cell=max(1,size//8)
    for y in range(8):
        for x in range(8): draw.rectangle((x*cell,y*cell,(x+1)*cell,(y+1)*cell),fill=(18,18,18) if (x+y)%2==0 else (238,238,238))
    image.save(path)

def generate_biome_bank(root,biome,card_size,terrain_size,seed):
    palette=palette_for(biome); d=root/"biomes"/biome.continent/biome.longitude/biome.altitude; d.mkdir(parents=True,exist_ok=True); save_terrain(d/"terrain.png",terrain_size,seed,palette); save_simple_noise(d/"bark.png",card_size,(.29,.19,.105),.03,seed); assets={}
    for category in ("trees","bushes","grass","fake_buildings"):
        key={"trees":"tree","bushes":"bush","grass":"grass","fake_buildings":"structures"}[category]; colors=palette[key]
        for spec in specs_for_biome(biome,category):
            path=d/f"{spec.id}.png"; color=colors[spec.variant_index%4]
            if category=="trees": save_tree_card(path,card_size,seed,color,spec.variant_index)
            elif category=="bushes": save_bush_card(path,card_size,seed,color,spec.variant_index)
            elif category=="grass": save_grass_card(path,card_size,seed,color,spec.variant_index)
            else: save_building_facade(path,card_size,seed,color,spec.variant_index,biome)
            assets[spec.id]=str(path.relative_to(root)).replace("\\","/")
    return {"biome":biome.id,"terrain":str((d/"terrain.png").relative_to(root)).replace("\\","/"),"bark":str((d/"bark.png").relative_to(root)).replace("\\","/"),"assets":assets}

def main():
    p=argparse.ArgumentParser(); p.add_argument("--config",required=True); p.add_argument("--seed",type=int,default=1995); ns=p.parse_args(); cp=Path(ns.config).resolve(); repo=cp.parents[3]; config=read_json(cp); out=repo/config["generated_dir"]/"textures"; shared=out/"shared"; shared.mkdir(parents=True,exist_ok=True); size=int(config.get("materials",{}).get("texture_size",256)); card=int(config.get("materials",{}).get("card_texture_size",128))
    save_asphalt(shared/"asphalt.png",size,ns.seed); save_simple_noise(shared/"guardrail.png",card,(.52,.54,.55),.025,ns.seed); save_checker(shared/"start_finish.png",card)
    manifests={b.id:generate_biome_bank(out,b,card,size,ns.seed) for b in supported_biomes()}; active=biome_from_config(config); m={"seed":ns.seed,"active_biome":active.id,"shared":{"asphalt":"shared/asphalt.png","guardrail":"shared/guardrail.png","start_finish":"shared/start_finish.png"},**manifests[active.id]}
    (out/"active_manifest.json").write_text(json.dumps(m,indent=2),encoding="utf-8"); (out/"bank_manifest.json").write_text(json.dumps({"seed":ns.seed,"biomes":manifests},indent=2),encoding="utf-8")
    print(f"[textures] generated {len(manifests)} South America biome combinations"); print("[textures] bank: 4 trees + 4 bushes + 4 grass cards + 4 facades per biome"); print(f"[textures] active={active.id} seed={ns.seed}"); print(f"[textures] wrote {out}"); return 0

if __name__=="__main__": raise SystemExit(main())
