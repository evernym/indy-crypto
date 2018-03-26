use bn::BigNumber;
use cl::*;
use cl::constants::{LARGE_E_START, ITERATION};
use cl::helpers::*;
use errors::IndyCryptoError;

use std::iter::FromIterator;
use utils::get_hash_as_int;

use authz::{AuthzProof, AuthzAccumulators};

/// Party that wants to check that prover has some credentials provided by issuer.
pub struct Verifier {}

impl Verifier {
    /// Creates and returns sub proof request entity builder.
    /// Part of proof request related to a particular schema-key.
    ///
    /// The purpose of sub proof request builder is building of sub proof request entity that
    /// represents requested attributes and predicates.
    ///
    /// # Example
    /// ```
    /// use indy_crypto::cl::verifier::Verifier;
    ///
    /// let mut sub_proof_request_builder = Verifier::new_sub_proof_request_builder().unwrap();
    /// sub_proof_request_builder.add_revealed_attr("name").unwrap();
    /// sub_proof_request_builder.add_predicate("age", "GE", 18).unwrap();
    /// let _sub_proof_request = sub_proof_request_builder.finalize().unwrap();
    /// ```
    pub fn new_sub_proof_request_builder() -> Result<SubProofRequestBuilder, IndyCryptoError> {
        let res = SubProofRequestBuilder::new()?;
        Ok(res)
    }

    /// Creates and returns proof verifier.
    ///
    /// The purpose of `proof verifier` is check proof provided by Prover.
    ///
    /// # Example
    /// ```
    /// use indy_crypto::cl::verifier::Verifier;
    ///
    /// let _proof_verifier = Verifier::new_proof_verifier().unwrap();
    /// ```
    pub fn new_proof_verifier() -> Result<ProofVerifier, IndyCryptoError> {
        Ok(ProofVerifier { credentials: BTreeMap::new() })
    }
}


#[derive(Debug)]
pub struct ProofVerifier {
    credentials: BTreeMap<String, VerifiableCredential>,
}

impl ProofVerifier {
    /// Add sub proof request to proof verifier.
    ///
    /// # Arguments
    /// * `proof_verifier` - Proof verifier.
    /// * `key_id` - unique credential identifier.
    /// * `credential_schema` - Credential schema.
    /// * `credential_pub_key` - Credential public key.
    /// * `rev_reg_pub` - Revocation registry public key.
    /// * `sub_proof_request` - Requested attributes and predicates instance pointer.
    ///
    /// #Example
    /// ```
    /// use indy_crypto::cl::issuer::Issuer;
    /// use indy_crypto::cl::verifier::Verifier;
    ///
    /// let mut credential_schema_builder = Issuer::new_credential_schema_builder().unwrap();
    /// credential_schema_builder.add_attr("sex").unwrap();
    /// let credential_schema = credential_schema_builder.finalize().unwrap();
    ///
    /// let (credential_pub_key, _credential_priv_key, _credential_key_correctness_proof) = Issuer::new_credential_def(&credential_schema, false).unwrap();
    ///
    /// let mut sub_proof_request_builder = Verifier::new_sub_proof_request_builder().unwrap();
    /// sub_proof_request_builder.add_revealed_attr("sex").unwrap();
    /// let sub_proof_request = sub_proof_request_builder.finalize().unwrap();
    ///
    /// let mut proof_verifier = Verifier::new_proof_verifier().unwrap();
    ///
    /// proof_verifier.add_sub_proof_request("issuer_key_id_1",
    ///                                      &sub_proof_request,
    ///                                      &credential_schema,
    ///                                      &credential_pub_key,
    ///                                      None,
    ///                                      None).unwrap();
    /// ```
    pub fn add_sub_proof_request(
        &mut self,
        key_id: &str,
        sub_proof_request: &SubProofRequest,
        credential_schema: &CredentialSchema,
        non_credential_schema_elements: &NonCredentialSchemaElements,
        credential_pub_key: &CredentialPublicKey,
        rev_key_pub: Option<&RevocationKeyPublic>,
        rev_reg: Option<&RevocationRegistry>,
    ) -> Result<(), IndyCryptoError> {
        ProofVerifier::_check_add_sub_proof_request_params_consistency(
            sub_proof_request,
            credential_schema,
        )?;

        self.credentials.insert(
            key_id.to_string(),
            VerifiableCredential {
                pub_key: credential_pub_key.clone()?,
                sub_proof_request: sub_proof_request.clone(),
                credential_schema: credential_schema.clone(),
                non_credential_schema_elements: non_credential_schema_elements.clone(),
                rev_key_pub: rev_key_pub.map(Clone::clone),
                rev_reg: rev_reg.map(Clone::clone),
            },
        );
        Ok(())
    }

    /// Verifies proof.
    ///
    /// # Arguments
    /// * `proof_verifier` - Proof verifier.
    /// * `proof` - Proof generated by Prover.
    /// * `nonce` - Nonce.
    ///
    ///
    /// #Example
    /// ```
    /// use indy_crypto::cl::new_nonce;
    /// use indy_crypto::cl::issuer::Issuer;
    /// use indy_crypto::cl::prover::Prover;
    /// use indy_crypto::cl::verifier::Verifier;
    ///
    /// let mut credential_schema_builder = Issuer::new_credential_schema_builder().unwrap();
    /// credential_schema_builder.add_attr("sex").unwrap();
    /// let credential_schema = credential_schema_builder.finalize().unwrap();
    ///
    /// let (credential_pub_key, credential_priv_key, cred_key_correctness_proof) = Issuer::new_credential_def(&credential_schema, false).unwrap();
    ///
    /// let master_secret = Prover::new_master_secret().unwrap();
    /// let master_secret_blinding_nonce = new_nonce().unwrap();
    /// let (blinded_master_secret, master_secret_blinding_data, blinded_master_secret_correctness_proof) =
    ///     Prover::blind_master_secret(&credential_pub_key, &cred_key_correctness_proof, &master_secret, &master_secret_blinding_nonce).unwrap();
    ///
    /// let mut credential_values_builder = Issuer::new_credential_values_builder().unwrap();
    /// credential_values_builder.add_value("sex", "5944657099558967239210949258394887428692050081607692519917050011144233115103").unwrap();
    /// let credential_values = credential_values_builder.finalize().unwrap();
    ///
    /// let credential_issuance_nonce = new_nonce().unwrap();
    ///
    /// let (mut credential_signature, signature_correctness_proof) =
    ///     Issuer::sign_credential("CnEDk9HrMnmiHXEV1WFgbVCRteYnPqsJwrTdcZaNhFVW",
    ///                             &blinded_master_secret,
    ///                             &blinded_master_secret_correctness_proof,
    ///                             &master_secret_blinding_nonce,
    ///                             &credential_issuance_nonce,
    ///                             &credential_values,
    ///                             &credential_pub_key,
    ///                             &credential_priv_key).unwrap();
    ///
    /// Prover::process_credential_signature(&mut credential_signature,
    ///                                      &credential_values,
    ///                                      &signature_correctness_proof,
    ///                                      &master_secret_blinding_data,
    ///                                      &master_secret,
    ///                                      &credential_pub_key,
    ///                                      &credential_issuance_nonce,
    ///                                      None, None, None).unwrap();
    ///
    /// let mut sub_proof_request_builder = Verifier::new_sub_proof_request_builder().unwrap();
    /// sub_proof_request_builder.add_revealed_attr("sex").unwrap();
    /// let sub_proof_request = sub_proof_request_builder.finalize().unwrap();
    ///
    /// let mut proof_builder = Prover::new_proof_builder().unwrap();
    /// proof_builder.add_sub_proof_request("issuer_key_id_1",
    ///                                     &sub_proof_request,
    ///                                     &credential_schema,
    ///                                     &credential_signature,
    ///                                     &credential_values,
    ///                                     &credential_pub_key,
    ///                                     None,
    ///                                     None).unwrap();
    ///
    /// let proof_request_nonce = new_nonce().unwrap();
    /// let proof = proof_builder.finalize(&proof_request_nonce, &master_secret).unwrap();
    ///
    /// let mut proof_verifier = Verifier::new_proof_verifier().unwrap();
    ///
    /// proof_verifier.add_sub_proof_request("issuer_key_id_1",
    ///                                      &sub_proof_request,
    ///                                      &credential_schema,
    ///                                      &credential_pub_key,
    ///                                      None,
    ///                                      None).unwrap();
    /// assert!(proof_verifier.verify(&proof, &proof_request_nonce).unwrap());
    /// ```
    pub fn verify(self,
                  proof: &Proof,
                  nonce: &Nonce,
                  accumulators: Option<&AuthzAccumulators>) -> Result<bool, IndyCryptoError> {
        trace!("ProofVerifier::verify: >>> proof: {:?}, nonce: {:?}", proof, nonce);

        ProofVerifier::_check_verify_params_consistency(&self.credentials, proof)?;

        let mut tau_list: Vec<Vec<u8>> = Vec::new();
        let mut include_authz_proof = false;

        for (issuer_key_id, proof_item) in &proof.proofs {
            let credential: &VerifiableCredential = &self.credentials[issuer_key_id];

            include_authz_proof |= credential.sub_proof_request.include_authz_proof;

            if let (Some(non_revocation_proof),
                    Some(cred_rev_pub_key),
                    Some(rev_reg),
                    Some(rev_key_pub)) =
                (
                    proof_item.non_revoc_proof.as_ref(),
                    credential.pub_key.r_key.as_ref(),
                    credential.rev_reg.as_ref(),
                    credential.rev_key_pub.as_ref(),
                )
            {
                tau_list.extend_from_slice(&ProofVerifier::_verify_non_revocation_proof(
                    &cred_rev_pub_key,
                    &rev_reg,
                    &rev_key_pub,
                    &proof.aggregated_proof.c_hash,
                    &non_revocation_proof,
                )?
                    .as_slice()?);
            };

            tau_list.append_vec(&ProofVerifier::_verify_primary_proof(
                &credential.pub_key.p_key,
                &proof.aggregated_proof.c_hash,
                &proof_item.primary_proof,
                &credential.credential_schema,
                &credential.non_credential_schema_elements,
                &credential.sub_proof_request,
            )?)?;
        }

        if include_authz_proof && proof.authz_proof.is_none() {
            return Ok(false);
        }

        let mut values: Vec<Vec<u8>> = Vec::new();

        values.extend_from_slice(&tau_list);
        values.extend_from_slice(&proof.aggregated_proof.c_list);


        if let Some(ref authz_proof) = proof.authz_proof {
            let t_list = authz_proof.verify(&proof.aggregated_proof.c_hash, accumulators.unwrap())?;
            values.push(t_list);
        }

        values.push(nonce.to_bytes()?);

        let c_hver = get_hash_as_int(&mut values)?;

        info!(target: "anoncreds_service", "Verifier verify proof -> done");

        let valid = c_hver == proof.aggregated_proof.c_hash;

        trace!("ProofVerifier::verify: <<< valid: {:?}", valid);

        Ok(valid)
    }

    fn _check_add_sub_proof_request_params_consistency(
        sub_proof_request: &SubProofRequest,
        cred_schema: &CredentialSchema,
    ) -> Result<(), IndyCryptoError> {
        trace!(
            "ProofVerifier::_check_add_sub_proof_request_params_consistency: >>> sub_proof_request: {:?}, cred_schema: {:?}",
            sub_proof_request,
            cred_schema
        );

        if sub_proof_request
            .revealed_attrs
            .difference(&cred_schema.attrs)
            .count() != 0
        {
            return Err(IndyCryptoError::InvalidStructure(
                format!("Claim doesn't contain requested attribute"),
            ));
        }

        let predicates_attrs = sub_proof_request
            .predicates
            .iter()
            .map(|predicate| predicate.attr_name.clone())
            .collect::<BTreeSet<String>>();

        if predicates_attrs.difference(&cred_schema.attrs).count() != 0 {
            return Err(IndyCryptoError::InvalidStructure(format!(
                "Claim doesn't contain attribute requested in predicate"
            )));
        }

        trace!("ProofVerifier::_check_add_sub_proof_request_params_consistency: <<<");

        Ok(())
    }

    fn _check_verify_params_consistency(
        credentials: &BTreeMap<String, VerifiableCredential>,
        proof: &Proof,
    ) -> Result<(), IndyCryptoError> {
        trace!(
            "ProofVerifier::_check_verify_params_consistency: >>> credentials: {:?}, proof: {:?}",
            credentials,
            proof
        );

        for (key_id, credential) in credentials {
            let proof_for_credential = proof.proofs.get(key_id.as_str()).ok_or(
                IndyCryptoError::AnoncredsProofRejected(format!("Proof not found")),
            )?;

            let proof_revealed_attrs = BTreeSet::from_iter(
                proof_for_credential
                    .primary_proof
                    .eq_proof
                    .revealed_attrs
                    .keys()
                    .cloned(),
            );

            if proof_revealed_attrs != credential.sub_proof_request.revealed_attrs {
                return Err(IndyCryptoError::AnoncredsProofRejected(format!(
                    "Proof revealed attributes not correspond to requested attributes"
                )));
            }

            let proof_predicates = proof_for_credential
                .primary_proof
                .ge_proofs
                .iter()
                .map(|ge_proof| ge_proof.predicate.clone())
                .collect::<BTreeSet<Predicate>>();

            if proof_predicates != credential.sub_proof_request.predicates {
                return Err(IndyCryptoError::AnoncredsProofRejected(format!(
                    "Proof predicates not correspond to requested predicates"
                )));
            }
        }

        trace!("ProofVerifier::_check_verify_params_consistency: <<<");

        Ok(())
    }

    fn _verify_primary_proof(
        p_pub_key: &CredentialPrimaryPublicKey,
        c_hash: &BigNumber,
        primary_proof: &PrimaryProof,
        cred_schema: &CredentialSchema,
        non_cred_schema_elements: &NonCredentialSchemaElements,
        sub_proof_request: &SubProofRequest,
    ) -> Result<Vec<BigNumber>, IndyCryptoError> {
        trace!(
            "ProofVerifier::_verify_primary_proof: >>> p_pub_key: {:?}, c_hash: {:?}, primary_proof: {:?}, cred_schema: {:?}, sub_proof_request: {:?}",
            p_pub_key,
            c_hash,
            primary_proof,
            cred_schema,
            sub_proof_request
        );

        let mut t_hat: Vec<BigNumber> = ProofVerifier::_verify_equality(
            p_pub_key,
            &primary_proof.eq_proof,
            c_hash,
            cred_schema,
            non_cred_schema_elements,
            sub_proof_request,
        )?;

        for ge_proof in primary_proof.ge_proofs.iter() {
            t_hat.append(&mut ProofVerifier::_verify_ge_predicate(
                p_pub_key,
                ge_proof,
                c_hash,
            )?)
        }

        trace!(
            "ProofVerifier::_verify_primary_proof: <<< t_hat: {:?}",
            t_hat
        );

        Ok(t_hat)
    }

    fn _verify_equality(
        p_pub_key: &CredentialPrimaryPublicKey,
        proof: &PrimaryEqualProof,
        c_hash: &BigNumber,
        cred_schema: &CredentialSchema,
        non_cred_schema_elements: &NonCredentialSchemaElements,
        sub_proof_request: &SubProofRequest,
    ) -> Result<Vec<BigNumber>, IndyCryptoError> {
        trace!(
            "ProofVerifier::_verify_equality: >>> p_pub_key: {:?}, proof: {:?}, c_hash: {:?}, cred_schema: {:?}, sub_proof_request: {:?}",
            p_pub_key,
            proof,
            c_hash,
            cred_schema,
            sub_proof_request
        );

        let unrevealed_attrs = cred_schema
            .attrs
            .union(&non_cred_schema_elements.attrs)
            .cloned()
            .collect::<BTreeSet<String>>()
            .difference(&sub_proof_request.revealed_attrs)
            .cloned()
            .collect::<BTreeSet<String>>();

        let t1: BigNumber = calc_teq(
            &p_pub_key,
            &proof.a_prime,
            &proof.e,
            &proof.v,
            &proof.m,
            &proof.m2,
            &unrevealed_attrs,
        )?;

        let mut ctx = BigNumber::new_context()?;

        let degree: BigNumber = BigNumber::from_u32(2)?.exp(
            &BigNumber::from_dec(
                &LARGE_E_START.to_string(),
            )?,
            Some(&mut ctx),
        )?;

        let mut rar = proof.a_prime.mod_exp(&degree, &p_pub_key.n, Some(&mut ctx))?;

        for (attr, encoded_value) in &proof.revealed_attrs {
            let cur_r = p_pub_key.r.get(attr).ok_or(
                IndyCryptoError::AnoncredsProofRejected(
                    format!("Value by key '{}' not found in pk.r", attr),
                ),
            )?;

            rar = cur_r
                .mod_exp(encoded_value, &p_pub_key.n, Some(&mut ctx))?
                .mod_mul(&rar, &p_pub_key.n, Some(&mut ctx))?;
        }

        let t2: BigNumber = p_pub_key
            .z
            .mod_div(&rar, &p_pub_key.n, Some(&mut ctx))?
            .inverse(&p_pub_key.n, Some(&mut ctx))?
            .mod_exp(&c_hash, &p_pub_key.n, Some(&mut ctx))?;

        let t: BigNumber = t1.mod_mul(&t2, &p_pub_key.n, Some(&mut ctx))?;

        trace!("ProofVerifier::_verify_equality: <<< t: {:?}", t);

        Ok(vec![t])
    }

    fn _verify_ge_predicate(
        p_pub_key: &CredentialPrimaryPublicKey,
        proof: &PrimaryPredicateGEProof,
        c_hash: &BigNumber,
    ) -> Result<Vec<BigNumber>, IndyCryptoError> {
        trace!(
            "ProofVerifier::_verify_ge_predicate: >>> p_pub_key: {:?}, proof: {:?}, c_hash: {:?}",
            p_pub_key,
            proof,
            c_hash
        );

        let mut ctx = BigNumber::new_context()?;
        let mut tau_list = calc_tge(
            &p_pub_key,
            &proof.u,
            &proof.r,
            &proof.mj,
            &proof.alpha,
            &proof.t,
        )?;

        for i in 0..ITERATION {
            let cur_t = proof.t.get(&i.to_string()).ok_or(
                IndyCryptoError::AnoncredsProofRejected(
                    format!("Value by key '{}' not found in proof.t", i),
                ),
            )?;

            tau_list[i] = cur_t
                .mod_exp(&c_hash, &p_pub_key.n, Some(&mut ctx))?
                .inverse(&p_pub_key.n, Some(&mut ctx))?
                .mod_mul(&tau_list[i], &p_pub_key.n, Some(&mut ctx))?;
        }

        let delta = proof.t.get("DELTA").ok_or(
            IndyCryptoError::AnoncredsProofRejected(
                format!("Value by key '{}' not found in proof.t", "DELTA"),
            ),
        )?;

        tau_list[ITERATION] = p_pub_key
            .z
            .mod_exp(
                &BigNumber::from_dec(&proof.predicate.value.to_string())?,
                &p_pub_key.n,
                Some(&mut ctx),
            )?
            .mul(&delta, Some(&mut ctx))?
            .mod_exp(&c_hash, &p_pub_key.n, Some(&mut ctx))?
            .inverse(&p_pub_key.n, Some(&mut ctx))?
            .mod_mul(&tau_list[ITERATION], &p_pub_key.n, Some(&mut ctx))?;

        tau_list[ITERATION + 1] =
            delta
                .mod_exp(&c_hash, &p_pub_key.n, Some(&mut ctx))?
                .inverse(&p_pub_key.n, Some(&mut ctx))?
                .mod_mul(&tau_list[ITERATION + 1], &p_pub_key.n, Some(&mut ctx))?;

        trace!(
            "ProofVerifier::_verify_ge_predicate: <<< tau_list: {:?},",
            tau_list
        );

        Ok(tau_list)
    }

    fn _verify_non_revocation_proof(
        r_pub_key: &CredentialRevocationPublicKey,
        rev_reg: &RevocationRegistry,
        rev_key_pub: &RevocationKeyPublic,
        c_hash: &BigNumber,
        proof: &NonRevocProof,
    ) -> Result<NonRevocProofTauList, IndyCryptoError> {
        trace!(
            "ProofVerifier::_verify_non_revocation_proof: >>> r_pub_key: {:?}, rev_reg: {:?}, rev_key_pub: {:?}, c_hash: {:?}",
            r_pub_key,
            rev_reg,
            rev_key_pub,
            c_hash
        );

        let ch_num_z = bignum_to_group_element(&c_hash)?;

        let t_hat_expected_values =
            create_tau_list_expected_values(r_pub_key, rev_reg, rev_key_pub, &proof.c_list)?;
        let t_hat_calc_values =
            create_tau_list_values(&r_pub_key, rev_reg, &proof.x_list, &proof.c_list)?;


        let non_revoc_proof_tau_list = Ok(NonRevocProofTauList {
            t1: t_hat_expected_values.t1.mul(&ch_num_z)?.add(
                &t_hat_calc_values
                    .t1,
            )?,
            t2: t_hat_expected_values.t2.mul(&ch_num_z)?.add(
                &t_hat_calc_values
                    .t2,
            )?,
            t3: t_hat_expected_values.t3.pow(&ch_num_z)?.mul(
                &t_hat_calc_values
                    .t3,
            )?,
            t4: t_hat_expected_values.t4.pow(&ch_num_z)?.mul(
                &t_hat_calc_values
                    .t4,
            )?,
            t5: t_hat_expected_values.t5.mul(&ch_num_z)?.add(
                &t_hat_calc_values
                    .t5,
            )?,
            t6: t_hat_expected_values.t6.mul(&ch_num_z)?.add(
                &t_hat_calc_values
                    .t6,
            )?,
            t7: t_hat_expected_values.t7.pow(&ch_num_z)?.mul(
                &t_hat_calc_values
                    .t7,
            )?,
            t8: t_hat_expected_values.t8.pow(&ch_num_z)?.mul(
                &t_hat_calc_values
                    .t8,
            )?,
        });

        trace!(
            "ProofVerifier::_verify_non_revocation_proof: <<< non_revoc_proof_tau_list: {:?}",
            non_revoc_proof_tau_list
        );

        non_revoc_proof_tau_list
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cl::prover;
    use cl::issuer;
    use cl::helpers::MockHelper;
    use cl::prover::mocks::*;

    #[test]
    fn sub_proof_request_builder_works() {
        let mut sub_proof_request_builder = Verifier::new_sub_proof_request_builder().unwrap();
        sub_proof_request_builder.add_revealed_attr("name").unwrap();
        sub_proof_request_builder
            .add_predicate("age", "GE", 18)
            .unwrap();
        let sub_proof_request = sub_proof_request_builder.finalize().unwrap();

        assert!(sub_proof_request.revealed_attrs.contains("name"));
        assert!(sub_proof_request.predicates.contains(
            &prover::mocks::predicate(),
        ));
    }

    //    #[test]
    //    fn verify_equality_works() {
    //        MockHelper::inject();
    //
    //        let proof = prover::mocks::eq_proof();
    //        let pk = issuer::mocks::credential_primary_public_key();
    //        let c_h = prover::mocks::aggregated_proof().c_hash;
    //        let credential_schema = prover::mocks::credential_schema();
    //
    //        let mut sub_proof_request_builder = SubProofRequestBuilder::new().unwrap();
    //        sub_proof_request_builder.add_revealed_attr("name").unwrap();
    //        let sub_proof_request = sub_proof_request_builder.finalize().unwrap();
    //
    //        let res: Vec<BigNumber> = ProofVerifier::_verify_equality(&pk,
    //                                                                  &proof,
    //                                                                  &c_h,
    //                                                                  &credential_schema,
    //                                                                  &sub_proof_request).unwrap();
    //
    //        assert_eq!("610975387630659407528754382495278179998808865971820131961970280432712404447935555966102422857540446384019466488097120691857122006661002884192894827783\
    //        537628810814237471341853389958293838330181411498429774548172099395542732810100523926895325520183827300354633217286905484099241454433444099585177676377082808935109\
    //        645554031301352772410507039140551301821108049643467491205117450921306244364744842209513914969770361271623495760542698907267864169959905991105301599435946991866298\
    //        98076989149707097243891475590010318619321486317753732474556827534548728195746464383092266373610988867273305094014679195413025534317874787564263", res[0].to_dec().unwrap());
    //    }

    #[test]
    fn _verify_ge_predicate_works() {
        MockHelper::inject();

        let proof = prover::mocks::ge_proof();
        let c_h = prover::mocks::aggregated_proof().c_hash;
        let pk = issuer::mocks::credential_primary_public_key();

        let res = ProofVerifier::_verify_ge_predicate(&pk, &proof, &c_h);

        assert!(res.is_ok());
        let res_data = res.unwrap();

        assert_eq!(
            "43225984033774157453915588728969954740922016588061170767759883574388114107958905\
                    80145510327781056576256091265658535791863565631921101568784161067523730940799382\
                    51087539730717568864171975952247517667106487450321170264768830591781523482230272\
                    84988406842946622985339420454211608186485285016155896999938463158447557656586276\
                    21689903896502209965457719725790268237515748857605562905832440747736039352987629\
                    81805829772159814948836823875044286617398373294016004168725609740915397028373509\
                    27621445128456248714727137147026050175191909894840982585618518292464739550751666\
                    403653903739045533820109066365142948601592388984410881602",
            res_data[0].to_dec().unwrap()
        );

        assert_eq!(
            "63167037026072942398294019215593337255694980292843024608055115132235566607178763\
                    14546898861554615034081633431633604405035150576524144051424624374195259279974757\
                    00360595769708911081770324532072772979411898396492309072199820574697296147120962\
                    94513242332072816961255542130215200023126389483528754364978393901072021530842534\
                    74110645907792296571682192970015963323753664843604342573947092753699976361360018\
                    17966196022093077001207834609335558528945702110901961641802045046423547768328171\
                    26437797625108121321799407666373473902472682987597187525494006271778138318478525\
                    631410977703522487793355681154461192306192992914175461194",
            res_data[4].to_dec().unwrap()
        );

        assert_eq!(
            "15546471718851723564857886862663935179794166626786498408624620729765508845105285\
                    97580551335718810885176188703476577917326031318426775939427674641170580110918908\
                    37364659297098189998980820065321498359485898580032941891554647642430051385534766\
                    52931362008973076385092585731812729399874743845396351910627204896770059152383405\
                    80373893666040974403074416923982771700232692567462481946906031736372978682762850\
                    09521094082831104454538037515053091806017267662389441717149551664439920986572452\
                    53197624135752050018130309700756072784623832910108501858472724468196046270864398\
                    453193009668829305398491649485593299490052238783007722867",
            res_data[5].to_dec().unwrap()
        );
    }
}
