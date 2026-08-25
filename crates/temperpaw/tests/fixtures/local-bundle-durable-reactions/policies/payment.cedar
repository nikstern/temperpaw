permit(
    principal is Agent,
    action == Action::"AuthorizePayment",
    resource is Payment
) when {
    principal.agent_type == "payment-service"
};
