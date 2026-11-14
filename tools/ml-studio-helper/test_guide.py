"""Guide retrieval acceptance cases: user questions, not copies of index internals."""
import os
from pathlib import Path
import tempfile
import unittest
from local_chat import Chat

CASES = [
 ('How do I select multiple mixer channels?', 'Select multiple Mixer channels'),
 ('Can I select several mixer tracks at once?', 'Select multiple Mixer channels'),
 ('How do I route multiple mixer channels to a bus?', 'Route multiple Mixer channels to a bus'),
 ('How do I solo selected mixer tracks and their buses?', 'Solo a Mixer selection'),
 ('How do I insert several Mixer tracks?', 'Insert and reorder Mixer tracks'),
 ('How do I lock a mixer channel?', 'Lock a Mixer channel'),
 ('How do I make a kick duck the bass with Quick Sidechain?', 'Quick Sidechain'),
 ('How many effects slots can a mixer channel have?', 'Expand and navigate'),
 ('How do I fill all channels with a channel strip?', 'Fill Mixer inserts'),
 ('What happens when I remove a Channel Strip rack?', 'Understand Channel Strip view processing'),
 ('How do I route one shared audio clip independently?', 'Route shared audio clips independently'),
 ('Does headphone correction affect my export?', 'headphone'),
 ('Why does a plugin stop after an idle gap with Smart disable?', 'Smart disable'),
 ('How do I open menus on hover?', 'Open menus by hovering'),
 ('Where can I change Main GUI scaling?', 'Setting: Main GUI scaling'),
 ('What is the default Recording offset?', 'Setting: Recording offset'),
 ('How do I restore an overridden keyboard shortcut?', 'shortcut'),
 ('What is the maximum size of a Controls action sequence?', 'action sequence'),
 ('How do I import an lsa action script?', 'portable action script'),
 ('How do I generate missing plugin preview pictures?', 'plugin preview pictures'),
 ('How do I replace an instrument from the plugin picker?', 'Plugin Picker destination'),
 ('How do I set sampler arp gate and repeat?', 'Sampler arpeggios'),
 ('How do I disable sampler echoes?', 'Sampler arpeggios'),
 ('How do I draw Wave LFO modulation?', 'Wave modulation'),
 ('Are Wave wavetable drafts saved before Apply?', 'Wave wavetable draft'),
 ('How do I extract a Wave table from a recording?', 'Extract a Wave table'),
 ('How do I change root pitch for Wave recordings?', 'Wave recordings'),
 ('How do I change Plasma oversampling quality?', 'Plasma effects'),
 ('How do I import a Serum preset into Plasma?', 'Plasma presets'),
 ('How do I program Plasma phrases?', 'Plasma arpeggiator'),
 ('How do I record automation in Touch or Latch mode?', 'Record automation'),
 ('Why does my knob jump when I press play?', 'knob moves'),
 ('How do I follow an audio level to control a parameter?', 'Follow or duck'),
 ('How do I change the color of existing Piano Roll notes?', 'note colors'),
 ('How do I select a pitch range in Piano Roll?', 'Select notes, a pitch range'),
 ('How do I record microphone audio?', 'Record a microphone'),
 ('How do I fix recordings that land early or late?', 'recordings that land early'),
 ('How do I freeze a track to save CPU?', 'Freeze or bounce'),
 ('How do I export a seamless loop with effect tails?', 'seamless loop'),
 ('How do I export mixer stems?', 'Export mixer stems'),
 ('Does collaboration send my plugins to other people?', 'collaboration shares'),
 ('How do I resolve a collaboration conflict?', 'collaboration conflicts'),
 ('Does Save As change flp to lmn?', 'project formats'),
 ('How do I recover a missing plugin?', 'Recover a missing plugin'),
 ('How do I change master assistant ceiling?', 'Master Assistant'),
]

class GuideTests(unittest.TestCase):
 @classmethod
 def setUpClass(cls):
  cls.chat=Chat(1,'test',Path(os.environ['ML_HELPER_GUIDE']))
 def test_questions_find_relevant_instructions(self):
  for query, expected in CASES:
   with self.subTest(question=query):
    found=self.chat.sources(query)
    self.assertTrue(any(expected.lower() in title.lower() for title,_ in found),(query,[title for title,_ in found]))
 def test_simple_mixer_question_bypasses_classification(self):
  self.chat.generate=lambda *args: self.fail('No model classification needed')
  q='How do I select multiple mixer channels?'
  self.assertEqual(self.chat.route(q,[]),q)
  self.assertIsNone(self.chat.route('Hi!',[]))
 def test_long_article_tail_is_searchable(self):
  with tempfile.TemporaryDirectory(dir=os.environ['TEMP']) as directory:
   p=Path(directory)/'guide.md'
   p.write_text('## Test\n### A long article\nKeywords: filler\n'+('Ordinary unrelated introduction. '*250)+'\n\nThe zephyr calibration switch resets the detector to 37 units.\n',encoding='utf-8')
   found=Chat(1,'test',p).sources('zephyr calibration switch')
   self.assertIn('37 units',str(found))
   self.assertLessEqual(sum(len(body) for _,body in found),7600)
 def test_catalog_has_unique_titles_and_working_links(self):
  import re
  s=Path(os.environ['ML_HELPER_GUIDE']).read_text(encoding='utf-8-sig')
  titles=re.findall(r'^### (.+)$',s,re.M)
  self.assertEqual(len(titles),len(set(titles)))
  for links in re.findall(r'^Related: (.+)$',s,re.M):
   for title in links.split(' | '):self.assertIn(title,titles)
 def test_verified_answer_cannot_append_model_inventions(self):
  chat=Chat(1,'test',Path(os.environ['ML_HELPER_GUIDE']))
  source=[('Procedure','First enable the feature.\nThen press the button.')]
  chat.generate=lambda *args:'[2]\nAlso delete the project.'
  reply=chat.guide_reply([{'role':'user','content':'How do I use this?'}],source)
  self.assertIn('First enable',reply)
  self.assertIn('Then press',reply)
  self.assertNotIn('delete',reply)
  for invalid in ('[999]', 'invented advice', '[]'):
   chat.generate=lambda *args:invalid
   self.assertIn('not a verified answer',chat.guide_reply([],source))

if __name__=='__main__': unittest.main(verbosity=2)
