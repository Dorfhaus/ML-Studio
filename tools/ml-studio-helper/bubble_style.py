"""Small cached sprite textures and speech-bubble geometry for the live UI."""
from PIL import Image, ImageDraw

SURFACE = '#141e30'
INPUT = '#1e2e47'

def surface_image(height, width=400, input_bar=False, focused=False):
    """Supersampled UI surfaces, cached by the caller; no per-frame rasterizing."""
    scale = 4
    image = Image.new('RGB', (width*scale, height*scale), '#010203' if not input_bar else SURFACE)
    draw = ImageDraw.Draw(image)
    def box(rect, radius, fill, outline):
        draw.rounded_rectangle(tuple(round(v*scale) for v in rect), radius*scale,
                               fill=fill, outline=outline, width=scale)
    if input_bar:
        box((1,1,width-1,height-1), min(20,height//2), INPUT, '#6098c3' if focused else '#354861')
    else:
        # A short gently curved tapered tail; the body hides its root seam.
        points=[]
        for i in range(25):
            t=i/24
            points.append(((1-t)**2*350+2*(1-t)*t*367+t*t*398,
                           (1-t)**2*88+2*(1-t)*t*111+t*t*118))
        for i in range(25):
            t=i/24
            points.append(((1-t)**2*398+2*(1-t)*t*369+t*t*350,
                           (1-t)**2*118+2*(1-t)*t*119+t*t*109))
        draw.polygon([(round(x*scale),round(y*scale)) for x,y in points], fill=SURFACE)
        draw.line([(round(x*scale),round(y*scale)) for x,y in points], fill='#3b536e',width=scale)
        box((4,4,358,height-4),26,SURFACE,'#3b536e')
        draw.rectangle((350*scale,91*scale,358*scale,107*scale),fill=SURFACE)
    return image.resize((width,height), Image.Resampling.LANCZOS)

def bubble_outline(height):
    # Repeated vertices keep straight edges and the pointed mouth-facing tail.
    return (24,4, 314,4, 338,4, 338,28,
            338,86, 338,86, 398,118, 398,118, 338,110, 338,110,
            338,height-28, 338,height-4, 314,height-4,
            24,height-4, 4,height-4, 4,height-28, 4,28, 4,4, 24,4)

def sprite_frames(path):
    """Resample once, not on animation ticks; use clean color-key silhouettes.

    Tk color-key windows cannot composite fractional alpha against the DAW.
    Convert the display-sized alpha to a solid silhouette and discard detached
    raster specks. This is runtime texture preparation; source art is untouched.
    """
    with Image.open(path) as source:
        sheet = source.convert('RGBA')
    width = sheet.width // 4
    frames = []
    for index in range(4):
        frame = sheet.crop((index*width, 0, (index+1)*width, sheet.height))
        frame.thumbnail((138, 242), Image.Resampling.LANCZOS)
        w, h = frame.size
        alpha = list(frame.getchannel('A').get_flattened_data())
        remaining = {i for i, a in enumerate(alpha) if a >= 160}
        largest = set()
        while remaining:
            seed = remaining.pop()
            component, pending = {seed}, [seed]
            while pending:
                pixel = pending.pop()
                x, y = pixel % w, pixel // w
                for yy in range(max(0,y-1), min(h,y+2)):
                    for xx in range(max(0,x-1), min(w,x+2)):
                        neighbor = yy*w+xx
                        if neighbor in remaining:
                            remaining.remove(neighbor)
                            component.add(neighbor)
                            pending.append(neighbor)
            if len(component) > len(largest):
                largest = component
        mask = Image.new('L', frame.size)
        mask.putdata([255 if i in largest else 0 for i in range(w*h)])
        frame.putalpha(mask)
        frames.append(frame)
    return frames
