use super::*;

fn bounds() -> Bounds {
    Bounds {
        timeout: Duration::from_secs(10),
        connect_timeout: Duration::from_secs(1),
        start_interval: Duration::from_secs(2),
        failure_cooldown: Duration::from_secs(60),
        max_response_bytes: 1024,
        raw_debug: false,
    }
}

#[tokio::test(start_paused = true)]
async fn timeout_cools_only_the_owning_provider() {
    let admission = Admission::new();
    let mut limits = bounds();
    limits.timeout = Duration::from_secs(1);
    let result = execute(
        &admission,
        &limits,
        Provider::BelgianNbb,
        Spacing::FromStart,
        async {
            tokio::time::sleep(Duration::from_secs(3)).await;
            Ok(())
        },
    )
    .await;
    assert!(matches!(
        result,
        Err(LookupError {
            kind: crate::model::LookupFailure::Registry {
                provider: Provider::BelgianNbb,
                error: RegistryFailure::ServiceUnavailable {
                    reason: UnavailabilityReason::Timeout,
                    ..
                }
            },
            ..
        })
    ));
    let result = execute(
        &admission,
        &limits,
        Provider::BelgianNbb,
        Spacing::FromStart,
        async {
            panic!("cooling client must not poll the request");
            #[allow(unreachable_code)]
            Ok(())
        },
    )
    .await;
    assert!(matches!(
        result,
        Err(LookupError {
            kind: crate::model::LookupFailure::Registry {
                error: RegistryFailure::CoolingDown { .. },
                ..
            },
            ..
        })
    ));
    assert!(
        execute(
            &Admission::new(),
            &limits,
            Provider::CzechAres,
            Spacing::FromStart,
            async { Ok(()) }
        )
        .await
        .is_ok()
    );
}

#[tokio::test(start_paused = true)]
async fn completion_spacing_preserves_two_request_adapter_gap() {
    for spacing in [Spacing::FromStart, Spacing::AfterCompletion] {
        let admission = Admission::new();
        let limits = bounds();
        execute(
            &admission,
            &limits,
            Provider::AustrianFirmenbuch,
            spacing,
            async {
                tokio::time::sleep(Duration::from_secs(4)).await;
                Ok(())
            },
        )
        .await
        .unwrap();
        let finished = Instant::now();
        execute(
            &admission,
            &limits,
            Provider::AustrianFirmenbuch,
            spacing,
            async { Ok(()) },
        )
        .await
        .unwrap();
        let expected = if spacing == Spacing::AfterCompletion {
            2
        } else {
            0
        };
        assert_eq!(Instant::now() - finished, Duration::from_secs(expected));
    }
}

#[tokio::test(start_paused = true)]
async fn occupied_slot_and_insufficient_admission_budget_do_not_send() {
    let admission = Admission::new();
    let limits = bounds();
    let mut guard = admission.lock().await;
    let result = execute(
        &admission,
        &limits,
        Provider::GreekGemi,
        Spacing::FromStart,
        async {
            panic!("occupied slot must not poll the request");
            #[allow(unreachable_code)]
            Ok(())
        },
    )
    .await;
    assert!(matches!(
        result,
        Err(LookupError {
            kind: crate::model::LookupFailure::Busy,
            ..
        })
    ));
    guard.next = Instant::now() + Duration::from_secs(11);
    drop(guard);
    let result = execute(
        &admission,
        &limits,
        Provider::GreekGemi,
        Spacing::FromStart,
        async {
            panic!("spacing beyond the deadline must not poll the request");
            #[allow(unreachable_code)]
            Ok(())
        },
    )
    .await;
    assert!(matches!(
        result,
        Err(LookupError {
            kind: crate::model::LookupFailure::Busy,
            ..
        })
    ));
}

#[test]
fn status_policy_preserves_provider_owned_misses_and_operational_failures() {
    assert!(matches!(
        check_status(404, None, &[200]),
        Err(RegistryFailure::Http { status: 404 })
    ));
    assert!(check_status(404, None, &[200, 404]).is_ok());
    // Even an erroneous accepted-status list must not bypass throttle/access policy.
    assert!(matches!(
        check_status(429, Some(RetryAfter::Seconds(17)), &[200, 429]),
        Err(RegistryFailure::RateLimited {
            retry_after: Some(RetryAfter::Seconds(17))
        })
    ));
    assert!(matches!(
        check_status(503, Some(RetryAfter::Seconds(23)), &[200]),
        Err(RegistryFailure::ServiceUnavailable {
            reason: UnavailabilityReason::ServerError,
            retry_after: Some(RetryAfter::Seconds(23))
        })
    ));
    assert!(matches!(
        check_status(408, None, &[200]),
        Err(RegistryFailure::ServiceUnavailable {
            reason: UnavailabilityReason::Timeout,
            ..
        })
    ));
    for status in [401, 403] {
        assert!(matches!(
            check_status(status, None, &[200, status]),
            Err(RegistryFailure::AccessDenied)
        ));
    }
    for status in [201, 204, 302] {
        assert!(
            matches!(check_status(status, None, &[200]), Err(RegistryFailure::Http { status: actual }) if actual == status)
        );
    }
}
