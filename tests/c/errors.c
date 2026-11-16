/*
 * C-interface error paths, licence-free (feature 005, task T016). Usage: errors <result file>
 * [leak]. Writes "ok" or the first failed check to the result file; writes nothing to standard
 * output or standard error. With "leak", deliberately does not free one error-message copy (the
 * positive control of the leak check).
 */
#include <math.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>

#include "optsolver.h"

static FILE *out;
#define CHECK(cond)                                                     \
    do {                                                                \
        if (!(cond)) {                                                  \
            fprintf(out, "errors.c:%d: %s\n", __LINE__, #cond);         \
            fclose(out);                                                \
            return 1;                                                   \
        }                                                               \
    } while (0)

/* The last message contains `needle`; the copy is freed. */
static int message_has(const char *needle) {
    char *m = optsolver_last_error_message();
    int ok = m != NULL && strstr(m, needle) != NULL;
    optsolver_string_free(m);
    return ok;
}

int main(int argc, char **argv) {
    if (argc < 2) return 2;
    out = fopen(argv[1], "w");
    if (!out) return 2;
    int leak = argc > 2 && strcmp(argv[2], "leak") == 0;

    CHECK(optsolver_abi_version() == ((OPTSOLVER_ABI_VERSION_MAJOR << 16) | OPTSOLVER_ABI_VERSION_MINOR));

    /* Freeing NULL is a no-op for every free function. */
    optsolver_env_free(NULL);
    optsolver_result_free(NULL);
    optsolver_caps_free(NULL);
    optsolver_string_free(NULL);

    /* Contained panic: internal error, message, the process continues, nothing printed. */
    CHECK(optsolver_test_panic() == OPTSOLVER_ERR_INTERNAL);
    CHECK(message_has("internal error"));
    CHECK(optsolver_last_error_solver_code() == 0);

    /* Unknown backend; NULL out-pointers and NULL objects. */
    optsolver_env *env = NULL;
    CHECK(optsolver_env_create(99, &env) == OPTSOLVER_ERR_INVALID && env == NULL);
    CHECK(message_has("backend"));
    CHECK(optsolver_env_create(OPTSOLVER_BACKEND_GUROBI, NULL) == OPTSOLVER_ERR_INVALID);
    CHECK(message_has("out"));
    optsolver_result *res = NULL;
    CHECK(optsolver_solve(NULL, NULL, NULL, &res) == OPTSOLVER_ERR_INVALID);
    CHECK(message_has("env"));
    CHECK(optsolver_solve_once(OPTSOLVER_BACKEND_GUROBI, NULL, NULL, &res) == OPTSOLVER_ERR_INVALID);
    CHECK(message_has("problem"));
    CHECK(optsolver_solve_once(OPTSOLVER_BACKEND_GUROBI, NULL, NULL, NULL) == OPTSOLVER_ERR_INVALID);
    CHECK(optsolver_solve_into(NULL, NULL, NULL, NULL, &res) == OPTSOLVER_ERR_INVALID);
    CHECK(message_has("buffers"));
    CHECK(optsolver_result_status(NULL) == 0);
    CHECK(optsolver_result_measure(NULL, 0, NULL) == OPTSOLVER_ERR_INVALID);
    CHECK(optsolver_caps_usable(NULL) == 0);
    CHECK(strcmp(optsolver_caps_reason(NULL), "") == 0);

    /* A problem structure too small for version 1.0. */
    size_t cp[1] = {0};
    optsolver_problem p;
    memset(&p, 0, sizeof p);
    p.struct_size = 8;
    p.A.col_ptrs = cp;
    p.sense = OPTSOLVER_SENSE_MINIMISE;
    uint64_t before = optsolver_environments_created();
    CHECK(optsolver_solve_once(OPTSOLVER_BACKEND_GUROBI, &p, NULL, &res) == OPTSOLVER_ERR_INVALID);
    CHECK(message_has("problem.struct_size"));

    /* Too large: rows above the solvers' limit, and columns above it with a one-element
     * col_ptrs (dimensions are checked before any array is read). */
    p.struct_size = sizeof p;
    p.A.rows = (size_t)1 << 31;
    CHECK(optsolver_solve_once(OPTSOLVER_BACKEND_GUROBI, &p, NULL, &res) == OPTSOLVER_ERR_TOO_LARGE);
    CHECK(message_has("m = 2147483648"));
    p.A.rows = 0;
    p.A.cols = (size_t)1 << 31;
    CHECK(optsolver_solve_once(OPTSOLVER_BACKEND_GUROBI, &p, NULL, &res) == OPTSOLVER_ERR_TOO_LARGE);
    CHECK(message_has("n = 2147483648"));
    CHECK(optsolver_environments_created() == before);

    /* Options: a truncated structure is accepted, the uncovered fields unset (threads would be
     * invalid if read); an invalid covered field is rejected. Both on the empty problem, which
     * is valid, so with no backend usable the call reaches environment creation only after the
     * options were accepted. */
    p.A.cols = 0;
    optsolver_options o;
    optsolver_options_init(&o);
    o.threads = -5;
    o.struct_size = (uint32_t)(offsetof(optsolver_options, time_limit) + sizeof(double));
    int32_t rc = optsolver_solve_once(99, &p, &o, &res);
    CHECK(rc == OPTSOLVER_ERR_INVALID);
    CHECK(message_has("backend")); /* past the options, at the backend */
    o.struct_size = sizeof o;
    CHECK(optsolver_solve_once(99, &p, &o, &res) == OPTSOLVER_ERR_INVALID);
    CHECK(message_has("options.threads"));
    o.threads = OPTSOLVER_UNSET;
    o.struct_size = 2;
    CHECK(optsolver_solve_once(99, &p, &o, &res) == OPTSOLVER_ERR_INVALID);
    CHECK(message_has("options.struct_size"));

    /* Message copies are per call and caller-owned. */
    char *m1 = optsolver_last_error_message();
    char *m2 = optsolver_last_error_message();
    CHECK(m1 != NULL && m2 != NULL && m1 != m2 && strcmp(m1, m2) == 0);
    optsolver_string_free(m1);
    if (!leak) optsolver_string_free(m2);

    /* Names. */
    CHECK(strcmp(optsolver_status_name(OPTSOLVER_STATUS_OPTIMAL), "Optimal") == 0);
    CHECK(strcmp(optsolver_limit_reason_name(OPTSOLVER_LIMIT_MEMORY), "Memory") == 0);
    CHECK(strcmp(optsolver_returned_name(OPTSOLVER_RETURNED_NONE), "None") == 0);
    CHECK(strcmp(optsolver_verification_name(OPTSOLVER_VERIFICATION_NOT_RUN), "NotRun") == 0);
    CHECK(strcmp(optsolver_cone_name(OPTSOLVER_CONE_ROTATED_QUADRATIC), "rotated quadratic") == 0);
    CHECK(strcmp(optsolver_status_name(42), "") == 0);

    fputs("ok\n", out);
    fclose(out);
    return 0;
}
