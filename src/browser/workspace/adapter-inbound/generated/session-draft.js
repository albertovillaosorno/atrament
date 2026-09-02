export function draftMutationHeaders(sessionSecret) {
    return {
        Authorization: `Bearer ${sessionSecret}`,
        "Content-Type": "text/plain; charset=utf-8",
    };
}
export function draftMutationTarget(field) {
    return `./api/session/${field}`;
}
