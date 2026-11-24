"""Synthetic prepared Bulgarian sources; no live or repository corpus reads."""
import json
import unittest
from bg_fixture_preparation import prepare
from bulstat_fixtures import page,HEADINGS
from registry_fixture_data import sha
import bg_fixtures as F


class BulgarianPreparationTests(unittest.TestCase):
    def test_contact_and_role_data_removed_with_source_hash_retained(self):
        raw=json.dumps({'deedStatus':2,'uic':'000000001','fullName':'Synthetic','uicWithCtx':'token',
            'sections':[{'subUICType':1,'subDeeds':[{'subUICType':1,'subDeedIsClosed':False,'groups':[{'fields':[
                {'fieldIdent':'00050','nameCode':'CR_F_5_L','fieldOperation':3,'htmlData':'<p>Държава: БЪЛГАРИЯ<br>ул. Пример 1<br>Телефон: secret<br>Адрес на електронна поща: private@example.invalid</p>'},
                {'fieldIdent':'00070','htmlData':'private officer'}]}]}]}]})
        review={'expected':{'taxId':'000000001','address.fullAddressByRegistry':'Държава: БЪЛГАРИЯ\nул. Пример 1\nТелефон: secret\nАдрес на електронна поща: private@example.invalid','registryDetails.managers':['private officer']}}
        result,bound,provenance=prepare(raw,review)
        self.assertNotIn('secret',result);self.assertNotIn('private',result);self.assertNotIn('token',result)
        self.assertNotIn('registryDetails.managers',bound['expected'])
        self.assertEqual(bound['expected']['address.fullAddressByRegistry'],'Държава: БЪЛГАРИЯ\nул. Пример 1')
        self.assertEqual(provenance['originalBodySha256'],sha(raw))

    def test_only_business_identifiers_can_become_fixture_filenames(self):
        self.assertTrue(F.valid_identifier('000000001'))
        self.assertTrue(F.valid_identifier('0000000010001'))
        for identifier in ('../outside','0000000000','１２３４５６７８９'):
            self.assertFalse(F.valid_identifier(identifier))

    def test_commercial_protocol_failure_is_not_an_access_blocker(self):
        c={'records':{},'history':{},'current':{}}
        row={'taxId':'000000001','checkedAt':'2026-11-24T00:00:00Z','outcome':'providerError','errorKind':'unexpectedResponse','serviceUnavailable':False}
        self.assertEqual(F.stage(c,row)['category'],'sourceFailure')
        row.update(errorKind='serviceUnavailable',serviceUnavailable=True)
        self.assertEqual(F.stage(c,row)['category'],'accessFailure')

    def test_commercial_negative_requires_captured_empty_success(self):
        corpus={'records':{},'history':{},'current':{}}
        row={'taxId':'000000001','checkedAt':'2026-11-24T00:00:00Z','outcome':'notFoundInCommercialRegister','rawResponse':'','transport':{'httpStatus':200},'error':None}
        record=F.stage(corpus,row)
        self.assertEqual(record['category'],'negative')
        self.assertEqual(record['collection']['rawResponse'],'')
        F.validate_negative(record)
        record['collection']['transport']['httpStatus']=503
        with self.assertRaises(AssertionError):F.validate_negative(record)
        row['transport']['httpStatus']=503
        with self.assertRaisesRegex(ValueError,'definitive empty successful'):F.stage(corpus,row)

    def test_bulstat_retains_search_name_in_correct_column(self):
        headers=''.join('<h1>'+s+'</h1>' for s in HEADINGS)
        source=headers+'<table><tr><td><a href="https://reports.bulstat.bg/bulstat-ireports/preview?uic=000000001&amp;__report=https%3A%2F%2Freports.bulstat.bg%2Fbulstat-ireports%2Fv_rpt1.rptdesign">000000001</a></td><td>Synthetic company</td></tr></table>'
        prepared=page(source,False)
        self.assertIn('<td>Synthetic company</td>',prepared)
        self.assertIn('href=',prepared)

    def test_bulstat_excludes_history_columns_and_unrelated_rows(self):
        headers=''.join('<h1>'+s+'</h1>' for s in HEADINGS)
        source=headers+'<table id="tab1"><tr><td>Код по БУЛСТАТ</td><td>000000001</td><td>private history</td><td>action token</td></tr><tr><td>Other</td><td>private officer</td><td></td><td></td></tr></table>'
        prepared=page(source,True)
        self.assertIn('000000001',prepared)
        self.assertNotIn('private',prepared);self.assertNotIn('token',prepared)


if __name__=='__main__':unittest.main()
