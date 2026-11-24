"""Minimize Icelandic fixture pages and PDF excerpts without keeping session data."""
import base64
import html
from html.parser import HTMLParser
import io
import json
import os
from pathlib import Path
import re
import subprocess
import sys
from urllib.parse import parse_qsl, urlencode, urlsplit, urlunsplit

VOID = {'area','base','br','col','embed','hr','img','input','link','meta','param','source','track','wbr'}

class Node:
    def __init__(self, tag='', attrs=()):
        self.tag=tag;self.attrs=dict(attrs);self.children=[]
    def walk(self, tag=None):
        if tag is None or self.tag==tag:yield self
        for child in self.children:
            if isinstance(child,Node):yield from child.walk(tag)
    def text(self):
        return ''.join(c.text() if isinstance(c,Node) else c for c in self.children)
    def render(self):
        attrs=''.join(' '+k+'="'+html.escape(v or '',quote=True)+'"' for k,v in sorted(self.attrs.items()) if k in ('class','id','type','name','value','action','data-typeid','data-itemid'))
        inner=''.join(c.render() if isinstance(c,Node) else html.escape(c) for c in self.children)
        return '<'+self.tag+attrs+'>'+('' if self.tag in VOID else inner+'</'+self.tag+'>')

class Tree(HTMLParser):
    def __init__(self, text):
        super().__init__(convert_charrefs=True);self.root=Node();self.stack=[self.root];self.feed(text)
    def handle_starttag(self, tag, attrs):
        n=Node(tag,attrs);self.stack[-1].children.append(n)
        if tag not in VOID:self.stack.append(n)
    def handle_endtag(self, tag):
        for i in range(len(self.stack)-1,0,-1):
            if self.stack[i].tag==tag:
                self.stack=self.stack[:i];break
    def handle_startendtag(self, tag, attrs):
        self.handle_starttag(tag,attrs)
        if tag not in VOID:self.handle_endtag(tag)
    def handle_data(self, data):self.stack[-1].children.append(data)

def primary(text):
    root=Tree(text).root;output=[]
    for company in root.walk('div'):
        if 'company' not in company.attrs.get('class','').split():continue
        parts=[]
        parts.extend(n.render() for n in company.walk('h1'))
        parts.extend(n.render() for n in company.walk('h2') if 'subtitle' in n.attrs.get('class','').split() and n.text().lstrip().startswith('Stofnað/Skráð:'))
        for table in company.walk('table'):
            headings=[n.text().strip() for head in table.walk('thead') for n in head.walk('th')]
            if headings==['Póstfang','Lögheimili','Sveitarfélag','Rekstrarform']:parts.append(table.render())
        parts.extend(n.render() for n in company.walk('button') if n.attrs.get('data-typeid')=='9')
        output.append('<div class="company">'+''.join(parts)+'</div>')
    return ''.join(output)

def overview(text, index):
    if index==0:
        value=json.loads(text)
        out={k:value[k] for k in ('addCartItemResult','shoppingCartUrl','cartItemCount') if k in value}
        if isinstance(out.get('shoppingCartUrl'),str):
            url=urlsplit(out['shoppingCartUrl']);pairs=parse_qsl(url.query)
            if len(pairs)!=1 or pairs[0][0]!='kid' or not re.fullmatch('[A-Za-z0-9]{1,64}',pairs[0][1]):
                raise ValueError('unreviewed Icelandic cart URL')
            out['shoppingCartUrl']=urlunsplit((url.scheme,url.netloc,url.path,urlencode({'kid':'FixtureCart'}),url.fragment))
        return json.dumps(out,ensure_ascii=False,sort_keys=True,separators=(',',':'))
    root=Tree(text).root;parts=[]
    if index in (1,3):
        for table in root.walk('table'):
            if table.attrs.get('id')=='MainContent_ucVoruGrid_GridView1':parts.append(table.render())
        for form in root.walk('form'):
            if form.attrs.get('id')!='ctl01':continue
            items=[]
            for n in form.walk('input'):
                name=n.attrs.get('name','')
                if name in ('__VIEWSTATE','__EVENTVALIDATION','ctl00$MainContent$ucVoruGrid$hfDownloadToken'):
                    if n.attrs.get('value'):n.attrs['value']='FixtureToken'
                    items.append(n.render())
                elif name in ('ctl00$MainContent$btnKaupa','ctl00$MainContent$ucVoruGrid$GridView1$ctl02$Btn_Saekja'):
                    # Only presence is consumed; no captured postback token is needed.
                    n.attrs['value']='';items.append(n.render())
            parts.append('<form id="ctl01">'+''.join(items)+'</form>')
    elif index==2:
        for form in root.walk('form'):
            fields=[n for n in form.walk('input') if n.attrs.get('type')=='hidden']
            if not any(n.attrs.get('name')=='amount' for n in fields):continue
            action=form.attrs.get('action','');url=urlsplit(action)
            pairs=parse_qsl(url.query)
            if len(pairs)!=1 or pairs[0][0]!='kid' or not re.fullmatch('[A-Za-z0-9]{1,64}',pairs[0][1]):
                raise ValueError('unreviewed Icelandic confirmation URL')
            names=[n.attrs.get('name') for n in fields]
            references=[n.attrs.get('value') for n in fields if n.attrs.get('name')=='ReferenceNumber']
            if len(names)!=len(set(names)) or references!=[pairs[0][1]]:
                raise ValueError('inconsistent Icelandic confirmation fields')
            form.attrs['action']=urlunsplit((url.scheme,url.netloc,url.path,urlencode({'kid':'FixtureCart'}),url.fragment))
            kept=[]
            for n in fields:
                if n.attrs.get('name')=='amount':kept.append(n)
                elif n.attrs.get('name')=='ReferenceNumber':
                    n.attrs['value']='FixtureCart';kept.append(n)
            form.children=kept;parts.append(form.render())
    return ''.join(parts)

def pdf_excerpt(encoded):
    """Independent Poppler extraction; generated excerpt contains no officer data."""
    executable=os.environ.get('REGISTRY_PDF_PYTHON',sys.executable)
    result=subprocess.run([executable,str(Path(__file__).resolve()),'pdf'],
                          input=encoded,text=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
    if result.returncode:
        raise RuntimeError('Icelandic PDF preparation failed (requires Poppler and ReportLab; REGISTRY_PDF_PYTHON may select another Python): '+result.stderr[-1000:])
    return result.stdout.strip()

def render_excerpt(encoded):
    from reportlab.pdfgen import canvas
    source=base64.b64decode(encoded,validate=True)
    result=subprocess.run(['pdftotext','-f','1','-l','1','-layout','-','-'],input=source,stdout=subprocess.PIPE,stderr=subprocess.PIPE,check=True)
    text=result.stdout.decode('utf-8')
    def single(pattern):
        values=re.findall(pattern,text,re.S)
        if len(values)!=1:raise ValueError('ambiguous PDF source excerpt')
        return values[0]
    date=single(r'Yfirlit\s+lögaðila\s+(\d{2}\.\d{2}\.\d{4})')
    identifier=single(r'Kt:\s*(\d{6}-\d{4})')
    status=single(r'Staða:\s*(.*?)\s*-\s*Stofnað/skráð:')
    if not status.strip() or len(status)>200:raise ValueError('unsupported PDF status')
    output=io.BytesIO();page=canvas.Canvas(output,invariant=1,pageCompression=0)
    page.setFont('Helvetica',12)
    for y,line in zip((780,750,725,700),('Prepared source excerpt - not an official document','Yfirlit lögaðila '+date,'Kt: '+identifier,'Staða: '+status.strip()+' - Stofnað/skráð:')):
        page.drawString(40,y,line)
    page.save()
    return base64.b64encode(output.getvalue()).decode('ascii')

if __name__=='__main__':
    if sys.argv[1:]!=['pdf']:raise SystemExit('internal PDF preparation only')
    print(render_excerpt(sys.stdin.read()))
