-- Starting vendors (checked by hand 2026-11-11). Add more with PUT /admin/sources/<id>.
INSERT OR IGNORE INTO sources (id, vendor, format, page_url, link_match, version_match, mode, note) VALUES
  ('spetool-vectric', 'SpeTool', 'vectric-vtdb',
   'https://spetools.com/pages/spetool-tool-file-database',
   'cdn\.shopify\.com/.*Vectric[^/]*\.vtdb', '(\d{8})\.vtdb', 'auto',
   'Only for SpeTool item numbers; the vendor warns other brands are dangerous with these values.'),
  ('idc-woodcraft-vectric', 'IDC Woodcraft', 'vectric-vtdb',
   'https://www.idcwoodcraft.com/pages/database-downloads',
   'filemonk\.io/.*Vectric[^/]*\.vtdb', NULL, 'auto',
   'File name never changes; new versions are found by the file''s fingerprint.'),
  ('amana-vectric', 'Amana Tool', 'vectric-vtdb',
   'https://www.amanatool.com/cnc-tool-databases-for-fusion-360tm-vectric',
   NULL, NULL, 'manual',
   'amanatool.com blocks automatic visitors; set the link by hand.'),
  ('amana-vectric-toolstoday', 'Amana Tool (ToolsToday copy)', 'vectric-tool',
   'https://toolstoday.com/learn/download-free-vectric-tool-files-for-amana-cnc-bits',
   'Amana-Tool-Vectric[^/]*\.tool$', '(\d{2}-\d{2}-\d{4})', 'auto',
   'Older than the file on Amana''s own site.');
