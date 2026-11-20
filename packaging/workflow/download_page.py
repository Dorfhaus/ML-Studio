"""Keep direct download buttons in sync with the platform manifests."""
import re
from urllib.parse import quote, urljoin

PLATFORMS = ('windows', 'linux', 'mac-arm64', 'mac-x86_64')


def render(page, manifests, base):
    """Replace only the nine direct hrefs, preserving all other published page bytes."""
    html = page.decode('utf-8')

    def replace_once(pattern, href):
        nonlocal html
        html, count = re.subn(pattern, lambda match: match[1] + href + '"', html)
        if count != 1:
            raise ValueError(f'Expected one matching download link, found {count}: {pattern}')

    def link(platform, key):
        manifest = manifests[platform]
        path = manifest[key]
        folder = ('mac' if platform.startswith('mac-') else
                  'updates' if key == 'source' else
                  'downloads' if platform == 'windows' else 'linux')
        if not re.fullmatch(rf'{folder}/{int(manifest["version"])}/[A-Za-z0-9 _.-]+', path) or '..' in path:
            raise ValueError(f'Invalid {platform} {key}: {path}')
        return urljoin(base, quote(path))

    for platform in PLATFORMS:
        group = rf'<div class="download-group" data-platform="{platform}">\s*'
        replace_once('(' + group + r'<a class="button" href=")[^"]*"', link(platform, 'url'))
        replace_once('(' + group + r'<a\b[^>]*>[^<]*</a>\s*<a class="source-link" href=")[^"]*"', link(platform, 'source'))
    replace_once(r'(<div id="source">\s*<a class="button secondary" href=")[^"]*"', link('windows', 'source'))
    return html.encode('utf-8')
