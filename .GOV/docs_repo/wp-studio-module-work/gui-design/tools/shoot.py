"""Renders a mock at 1920x1080 with a headless Edge under a private temp profile, and runs the mechanical
checks from acceptance-checks.json over its DOM. Usage: shoot.py <out_dir> <mock.html> [...]

The browser process is started and left to exit by itself; no other browser process is touched."""
import http.server
import json
import os
import subprocess
import sys
import tempfile
import threading
import time

EDGE = r"C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe"
HERE = os.path.dirname(os.path.abspath(__file__))

CHECK_JS = r"""
<script>
(function(){
 var out={clipped:[],overflow_x:[],off_grid:[],small_text:[],elements:0};
 var all=document.body.querySelectorAll('*');
 out.elements=all.length;
 function vis(e){var r=e.getBoundingClientRect();return r.width>0&&r.height>0;}
 for(var i=0;i<all.length;i++){var e=all[i];
  if(e.closest('svg')||e.tagName==='STYLE')continue;
  var cs=getComputedStyle(e);
  var own=[].some.call(e.childNodes,function(n){return n.nodeType===3&&n.textContent.trim().length>0;});
  if(own){
   if(parseFloat(cs.fontSize)<11 && vis(e)) out.small_text.push(e.textContent.trim().slice(0,30));
   if(e.scrollWidth>e.clientWidth+1 && cs.display!=='inline' && e.clientWidth>0){
     out.clipped.push({t:e.textContent.trim().slice(0,40),c:e.className,sw:e.scrollWidth,cw:e.clientWidth});
   }
   if(cs.display==='inline'){var pr=e.getBoundingClientRect(),p=e.parentElement.getBoundingClientRect();
     if(pr.right>p.right+1){out.overflow_x.push({t:e.textContent.trim().slice(0,40),over:Math.round(pr.right-p.right),c:e.parentElement.className});}}
  }
  if(e.classList.contains('dock')||e.classList.contains('grp')||e.classList.contains('sl')||e.classList.contains('sv')||e.classList.contains('pr')){
    var r=e.getBoundingClientRect();
    var inBar=e.closest('.cb');if((!inBar&&r.left%1)||r.top%1||r.width%1||r.height%1) out.off_grid.push({c:e.className,r:[r.left,r.top,r.width,r.height]});
  }
 }
 // container-level overflow: a row or bar whose children run past it
 var boxes=document.querySelectorAll('.pr,.cb,.ts,.mb,.gh,.st,.lr,.tree,.mi,.foot,.rail-b,.actbar');
 out.row_overflow=[];
 for(var j=0;j<boxes.length;j++){var b=boxes[j];var br=b.getBoundingClientRect();var over=0;
   for(var k=0;k<b.children.length;k++){var c=b.children[k].getBoundingClientRect();if(c.width>0)over=Math.max(over,c.right-br.right);}
   if(over>0.5)out.row_overflow.push({c:b.className,t:b.textContent.trim().slice(0,40),over:Math.round(over)});}
 var regs=[].slice.call(document.querySelectorAll('.mb,.ts,.cb,.tr,.st,.dock,.rail-l,.rail-b')).map(function(e){return {c:e.className,r:e.getBoundingClientRect()};});
 out.region_overlap=[];
 for(var a=0;a<regs.length;a++)for(var b2=a+1;b2<regs.length;b2++){var A=regs[a].r,B=regs[b2].r;var ow=Math.min(A.right,B.right)-Math.max(A.left,B.left),oh=Math.min(A.bottom,B.bottom)-Math.max(A.top,B.top);if(ow>0&&oh>0)out.region_overlap.push({a:regs[a].c,b:regs[b2].c,area:ow*oh});}
 __POST__
})();
</script>
"""


class H(http.server.BaseHTTPRequestHandler):
    result = {}
    pages = {}

    def do_GET(self):
        body = H.pages.get(self.path.strip("/"), b"")
        self.send_response(200)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_POST(self):
        n = int(self.headers.get("Content-Length", 0))
        H.result[self.path.strip("/")] = json.loads(self.rfile.read(n).decode("utf-8"))
        self.send_response(204)
        self.end_headers()

    def log_message(self, *a):
        pass


def wait_for(fn, seconds):
    end = time.time() + seconds
    while time.time() < end:
        if fn():
            return True
        time.sleep(0.25)
    return False


def run(out_dir, path, port):
    base = os.path.splitext(os.path.basename(path))[0]
    prof = tempfile.mkdtemp(prefix="edge-prof-", dir=out_dir)
    png = os.path.join(out_dir, base + ".png")
    if os.path.exists(png):
        os.remove(png)
    url = "file:///" + os.path.abspath(path).replace("\\", "/").replace(" ", "%20")
    common = [EDGE, "--headless=new", "--disable-gpu", "--hide-scrollbars", "--no-first-run", "--disable-extensions", "--user-data-dir=" + prof, "--window-size=1920,1080", "--force-device-scale-factor=1"]
    subprocess.run(common + ["--screenshot=" + png, url], capture_output=True, timeout=60)
    shot = wait_for(lambda: os.path.exists(png) and os.path.getsize(png) > 0, 40)
    js = CHECK_JS.replace("__POST__", "fetch('/%s',{method:'POST',body:JSON.stringify(out)});" % base)
    html_ = open(path, encoding="utf-8").read().replace("</body>", js + "</body>")
    H.pages[base] = html_.encode("utf-8")
    png2 = os.path.join(out_dir, base + ".check.png")
    turl = "http://127.0.0.1:%d/%s" % (port, base)
    subprocess.run(common + ["--screenshot=" + png2, turl], capture_output=True, timeout=60)
    wait_for(lambda: base in H.result, 40)
    res = H.result.get(base, {"error": "no result"})
    print(base, "shot" if shot else "NO SHOT", {k: (len(v) if isinstance(v, list) else v) for k, v in res.items()})
    for k in ("clipped", "overflow_x", "row_overflow", "off_grid", "small_text", "region_overlap"):
        for row in res.get(k, [])[:40]:
            print("  ", k, row)


if __name__ == "__main__":
    srv = http.server.HTTPServer(("127.0.0.1", 0), H)
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    for p in sys.argv[2:]:
        run(sys.argv[1], p, srv.server_port)
