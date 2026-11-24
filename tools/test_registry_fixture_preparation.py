"""Synthetic preparation checks; no collected data, providers or PDF dependencies."""
import json
import unittest
from unittest.mock import patch

import registry_fixture_data as D
import registry_fixture_html as H
from registry_fixture_projection import project


class PreparationTests(unittest.TestCase):
    def test_json_projection_preserves_identifier_and_removes_unused_contacts(self):
        value={'success':True,'result':{'records':[{'company_num':'000123',
               'company_name':'Synthetic','company_status':'Normal','email':'private@example.invalid'}]}}
        out=project('IE',value)
        self.assertEqual(out['result']['records'][0]['company_num'],'000123')
        self.assertNotIn('email',out['result']['records'][0])
        self.assertIn('email',value['result']['records'][0])

    def test_polish_officer_counts_preserve_lifecycle_without_personal_details(self):
        value={'odpis':{'dane':{'dzial6':{'likwidacja':{'likwidatorzy':[
            {'imie':'Private','pesel':'secret'}, {'imie':'Second'}]}}}}}
        out=project('PL',value)
        self.assertEqual(out['odpis']['dane']['dzial6']['likwidacja']['likwidatorzy'],[{},{}])
        self.assertNotIn('secret',json.dumps(out))
        with self.assertRaisesRegex(ValueError,'unreviewed Polish lifecycle'):
            project('PL',{'odpis':{'dane':{'dzial6':{'unknownLifecycle':True}}}})

    def test_nested_capture_hashes_bind_prepared_lifecycle(self):
        raw={'id':'000123','checkedAt':'2026-01-01T00:00:00Z','capture':{
            'httpStatus':404,'body':'not found','association':{'httpStatus':200,
            'body':json.dumps({'odpis':{'naglowekA':{'numerKRS':'000123'},'private':'secret'}})}}}
        row,review,provenance=D.prepare('PL',raw,{'fields':{'/taxId':'000123'}})
        body=row['capture']['association']['body']
        self.assertEqual(review['bodySha256'],D.sha(body))
        self.assertEqual(review['captureSha256'],D.digest(row['capture']))
        self.assertEqual(provenance['originalObservationSha256'],D.digest(raw))
        self.assertNotIn('secret',D.canonical(row))

    def test_iceland_selects_company_facts_without_other_page_sections(self):
        html='<div class="company"><h1>Synthetic 123456-7890</h1><p>unused private details</p><h2 class="subtitle">Stofnað/Skráð: 01.01.2000</h2><button data-typeid="9" data-itemid="1234567890">Overview</button></div>'
        prepared=H.primary(html)
        self.assertIn('123456-7890',prepared)
        self.assertNotIn('private',prepared)
        self.assertIn('Stofnað/Skráð:',prepared)

    def test_iceland_redacts_tokens_consistently(self):
        value=H.overview(json.dumps({'shoppingCartUrl':'https://example.invalid/cart?kid=Secret123','cartItemCount':1,'unused':'private'}),0)
        self.assertIn('FixtureCart',value)
        self.assertNotIn('Secret123',value)
        self.assertNotIn('private',value)
        page='<form id="ctl01"><input name="__VIEWSTATE" value="secret"><input name="unused" value="private"></form>'
        value=H.overview(page,1)
        self.assertIn('FixtureToken',value)
        self.assertNotIn('secret',value)
        self.assertNotIn('private',value)
        with self.assertRaises(ValueError):
            H.overview('{"shoppingCartUrl":"https://example.invalid/cart?unexpected=value"}',0)

    def test_redaction_refuses_to_hide_mismatched_confirmation_tokens(self):
        source='<form action="https://example.invalid/return?kid=Original"><input type="hidden" name="amount" value="0"><input type="hidden" name="ReferenceNumber" value="Wrong"></form>'
        with self.assertRaisesRegex(ValueError,'inconsistent'):H.overview(source,2)

    def test_partial_capture_retains_typed_access_failure(self):
        raw={'id':'000123','capture':{'httpStatus':200,'body':'<div class="company"><h1>Synthetic</h1></div>'},
             'lookupError':{'isServiceUnavailable':True,'details':{'provider':'IcelandicSkatturinn','reason':'connection','debug':'private'}}}
        row,_,_=D.prepare('IS',raw,{'fields':{}})
        self.assertEqual(row['collectionFailure'],{'provider':'IcelandicSkatturinn','reason':'connection'})
        self.assertNotIn('private',D.canonical(row))

    def test_pdf_preparation_uses_separate_extractor_and_preserves_original_hash(self):
        raw={'id':'000123','capture':{'httpStatus':200,'body':'<div class="company"><h1>Synthetic</h1></div>',
             'overviewResponses':[None,None,None,None,{'httpStatus':200,'encoding':'base64','body':'original-pdf'}]}}
        with patch.object(H,'pdf_excerpt',return_value='prepared-pdf'):
            row,review,provenance=D.prepare('IS',raw,{'fields':{}})
        self.assertEqual(row['capture']['overviewResponses'][4]['body'],'prepared-pdf')
        self.assertEqual(provenance['originalBodyHashes']['overview4'],D.sha('original-pdf'))
        self.assertEqual(review['captureSha256'],D.digest(row['capture']))


if __name__=='__main__':unittest.main()
