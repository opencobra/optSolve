/*
 * Link demonstration (feature 005, task T018, success criterion SC-007): the LP toy instance
 * toy_lp_max (tests/fixtures/small/toy_lp_max.mps) solved through the C interface, built with only
 * the flags docs/llm-interface.md states. maximise x1 + 2 x2 s.t. x1 + x2 <= 4, x1 + 3 x2 <= 6,
 * x >= 0; optimum 5 at (3, 1). Usage: minimal gurobi|mosek. Prints the status, objective, x and
 * verification verdict; exit code 0 when Optimal, objective 5 and Verified.
 */
#include <math.h>
#include <stdio.h>
#include <string.h>

#include "optsolver.h"

int main(int argc, char **argv) {
    int32_t backend = argc > 1 && strcmp(argv[1], "mosek") == 0 ? OPTSOLVER_BACKEND_MOSEK : OPTSOLVER_BACKEND_GUROBI;
    if (optsolver_abi_version() >> 16 != OPTSOLVER_ABI_VERSION_MAJOR) return 2;
    size_t col_ptrs[] = {0, 2, 4}, row_indices[] = {0, 1, 0, 1};
    double values[] = {1, 1, 1, 3}, c[] = {1, 2}, l[] = {0, 0}, u[] = {INFINITY, INFINITY};
    char row_sense[] = {'<', '<'};
    double b[] = {4, 6};
    optsolver_problem p;
    memset(&p, 0, sizeof p);
    p.struct_size = sizeof p;
    p.A.rows = 2;
    p.A.cols = 2;
    p.A.col_ptrs = col_ptrs;
    p.A.row_indices = row_indices;
    p.A.values = values;
    p.c = c;
    p.sense = OPTSOLVER_SENSE_MAXIMISE;
    p.l = l;
    p.u = u;
    p.row_sense = row_sense;
    p.b = b;
    optsolver_env *env = NULL;
    if (optsolver_env_create(backend, &env) != OPTSOLVER_OK) {
        char *m = optsolver_last_error_message();
        printf("environment: %s\n", m ? m : "");
        optsolver_string_free(m);
        return 1;
    }
    optsolver_result *r = NULL;
    int32_t rc = optsolver_solve(env, &p, NULL, &r);
    if (rc != OPTSOLVER_OK) {
        char *m = optsolver_last_error_message();
        printf("solve: %s\n", m ? m : "");
        optsolver_string_free(m);
        optsolver_env_free(env);
        return 1;
    }
    double obj = NAN;
    optsolver_result_objective(r, &obj);
    size_t n = 0;
    const double *x = optsolver_result_array(r, OPTSOLVER_ARRAY_X, &n);
    int32_t verdict = optsolver_result_verification(r);
    printf("%s %s: status %s, objective %.6g, x = (%.6g, %.6g), verification %s\n",
           optsolver_result_solver_name(r), optsolver_result_solver_version(r),
           optsolver_status_name(optsolver_result_status(r)), obj, n > 0 ? x[0] : NAN,
           n > 1 ? x[1] : NAN, optsolver_verification_name(verdict));
    int ok = optsolver_result_status(r) == OPTSOLVER_STATUS_OPTIMAL && fabs(obj - 5) < 1e-6 &&
             verdict == OPTSOLVER_VERIFICATION_VERIFIED;
    optsolver_result_free(r);
    optsolver_env_free(env);
    return ok ? 0 : 1;
}
