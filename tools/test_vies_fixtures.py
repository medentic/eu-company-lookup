"""Synthetic VIES history and independent source review checks."""
import copy
import unittest

import vies_fixtures as F
from vies_evidence import expected_hash


def observation(identifier='DE123456789',valid='true',name='---',address='---'):
    soap=f'<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body><checkVatResponse xmlns="urn:ec.europa.eu:taxud:vies:services:checkVat:types"><countryCode>{identifier[:2]}</countryCode><vatNumber>{identifier[2:]}</vatNumber><requestDate>2026-11-24+01:00</requestDate><valid>{valid}</valid><name>{name}</name><address>{address}</address></checkVatResponse></s:Body></s:Envelope>'
    return {'vatId':identifier,'checkedAt':'2026-11-24T00:00:00Z','attempts':[{'httpStatus':200,'rawBody':soap,'response':{'vatId':identifier,'valid':valid=='true','company':None if name=='---' else name,'address':{'fullAddressByVies':address}}}]}


def corpus():
    corpus=F.empty('DE')
    case={'vatId':'DE123456789','valid':True,'viesOutput':{'company':None,'address':'---'},'expected':{'address':{'city':None}}}
    F.append(corpus,{'vatId':case['vatId'],'observedAt':'','category':'reviewed','case':case,'review':{'expectedSha256':expected_hash(case['expected'])}})
    return corpus


class ViesFixtureTests(unittest.TestCase):
    def test_source_preserves_original_address_and_meaningful_name(self):
        row=observation(name=' Name ',address='\nRaw address\n')
        self.assertEqual(F.source_from_soap(row),{'valid':True,'company':' Name ','address':'\nRaw address\n'})
        self.assertEqual(F.source_from_soap(observation())['address'],'---')

    def test_empty_present_address_is_distinct_from_missing_address(self):
        row=observation(valid='false',address='')
        self.assertEqual(F.source_from_soap(row)['address'],'')
        row['attempts'][0]['rawBody']=row['attempts'][0]['rawBody'].replace('<address></address>','')
        row['attempts'][0]['response']['address']['fullAddressByVies']=None
        self.assertIsNone(F.source_from_soap(row)['address'])

    def test_matching_source_carries_review_without_using_parser_address(self):
        c=corpus();row=observation();row['attempts'][0]['response']['address']['city']='wrong parser value'
        record=F.stage_row(c,row)
        self.assertEqual(record['case']['expected']['address']['city'],None)
        self.assertEqual(record['category'],'reviewed')

    def test_changes_and_errors_are_separate_and_history_is_immutable(self):
        c=corpus();original=copy.deepcopy(c['records'])
        changed=F.stage_row(c,observation(address='new address'))
        self.assertEqual(changed['category'],'unresolved');self.assertNotIn('case',changed)
        F.append(c,changed)
        failed=F.stage_row(c,{'vatId':'DE123456789','checkedAt':'2026-11-25T00:00:00Z','attempts':[{'error':{'code':'totalDeadlineExceeded'}}]})
        self.assertEqual(failed['category'],'accessFailure');F.append(c,failed)
        F.validate(c)
        self.assertTrue(all(c['records'][k]==v for k,v in original.items()))
        self.assertEqual(F.coverage(c)['latest'],{'accessFailure':1})
        key=next(iter(c['records']));c['records'][key]['category']='unresolved'
        with self.assertRaises(AssertionError):F.validate(c)

    def test_protocol_failures_are_not_access_blockers_or_negative_vat(self):
        row={'vatId':'DE123456789','checkedAt':'2026-11-25T00:00:00Z','attempts':[{'error':{'code':'mismatchedResponse'}}]}
        record=F.stage_row(corpus(),row)
        self.assertEqual(record['category'],'sourceFailure')
        self.assertNotIn('case',record)

    def test_soap_identity_and_recorded_validity_must_match(self):
        row=observation();row['vatId']='DE987654321'
        with self.assertRaisesRegex(ValueError,'identifier mismatch'):F.source_from_soap(row)
        row=observation();row['attempts'][0]['response']['valid']=False
        with self.assertRaisesRegex(ValueError,'disagree'):F.source_from_soap(row)


if __name__=='__main__':unittest.main()
