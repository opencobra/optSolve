/*
 * Generic C driver for the C-interface tests (feature 005, task T003). Reads a problem file
 * written by tests/common/cdata.rs, solves it through the C interface only, and writes every
 * result field to a result file. Usage: driver <problem file> <result file>.
 *
 * File format (both directions): one item per line, "key count v1 v2 ...", numbers in %.17g
 * (inf, -inf and nan allowed); strings as "key 1 text-to-end-of-line". Writes nothing to standard
 * output or standard error (the runner checks).
 */
#include <math.h>
#include <pthread.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "optsolver.h"

#define MAXKEYS 128

typedef struct item {
    char key[64];
    size_t n;
    double *v;
    char *text;
} item;

/* Every allocation of the driver is tracked and freed at exit, so a leak check sees only the
 * library's allocations. */
static void *allocs[4096];
static size_t nallocs;
static void *track(void *p) {
    if (nallocs < sizeof allocs / sizeof allocs[0]) allocs[nallocs++] = p;
    return p;
}
static void free_all(void) {
    for (size_t i = 0; i < nallocs; i++) free(allocs[i]);
    nallocs = 0;
}

static item items[MAXKEYS];
static size_t nitems;

static void die(const char *what) {
    FILE *f = fopen("driver-failure.txt", "w");
    if (f) {
        fputs(what, f);
        fclose(f);
    }
    exit(3);
}

static void read_file(const char *path) {
    FILE *f = fopen(path, "r");
    if (!f) die("cannot open problem file");
    char key[64];
    while (fscanf(f, "%63s", key) == 1) {
        if (nitems == MAXKEYS) die("too many keys");
        item *it = &items[nitems++];
        strcpy(it->key, key);
        unsigned long long n = 0;
        if (fscanf(f, "%llu", &n) != 1) die("bad count");
        it->n = (size_t)n;
        if (strncmp(key, "str_", 4) == 0) {
            char line[4096];
            int c = fgetc(f); /* the separating space */
            (void)c;
            if (!fgets(line, sizeof line, f)) line[0] = 0;
            line[strcspn(line, "\n")] = 0;
            it->text = (char *)track(strdup(line));
            continue;
        }
        it->v = (double *)track(malloc((it->n ? it->n : 1) * sizeof(double)));
        for (size_t i = 0; i < it->n; i++) {
            char tok[64];
            if (fscanf(f, "%63s", tok) != 1) die("bad value");
            it->v[i] = strtod(tok, NULL);
        }
    }
    fclose(f);
}

static item *get(const char *key) {
    for (size_t i = 0; i < nitems; i++)
        if (strcmp(items[i].key, key) == 0) return &items[i];
    return NULL;
}

static double scalar(const char *key, double dflt) {
    item *it = get(key);
    return it && it->n ? it->v[0] : dflt;
}

static const double *dbl(const char *key) {
    item *it = get(key);
    return it && it->n ? it->v : NULL;
}

static size_t *sizes(const char *key) {
    item *it = get(key);
    if (!it || !it->n) return NULL;
    size_t *s = (size_t *)track(malloc(it->n * sizeof(size_t)));
    for (size_t i = 0; i < it->n; i++) s[i] = (size_t)it->v[i];
    return s;
}

static int32_t *ints32(const char *key) {
    item *it = get(key);
    if (!it || !it->n) return NULL;
    int32_t *s = (int32_t *)track(malloc(it->n * sizeof(int32_t)));
    for (size_t i = 0; i < it->n; i++) s[i] = (int32_t)it->v[i];
    return s;
}

static FILE *out;

static void put(const char *key, size_t n, const double *v) {
    fprintf(out, "%s %zu", key, n);
    for (size_t i = 0; i < n; i++) fprintf(out, " %.17g", v[i]);
    fputc('\n', out);
}

static void put1(const char *key, double v) { put(key, 1, &v); }

static void puts_(const char *key, const char *s) { fprintf(out, "str_%s 1 %s\n", key, s ? s : ""); }

static void write_error(int32_t rc) {
    put1("rc", rc);
    char *m = optsolver_last_error_message();
    puts_("message", m ? m : "");
    optsolver_string_free(m);
    put1("vendor_code", optsolver_last_error_solver_code());
}

static void write_result(const optsolver_result *r) {
    put1("status", optsolver_result_status(r));
    puts_("status_name", optsolver_status_name(optsolver_result_status(r)));
    put1("limit_reason", optsolver_result_limit_reason(r));
    puts_("limit_reason_name", optsolver_limit_reason_name(optsolver_result_limit_reason(r)));
    put1("returned", optsolver_result_returned(r));
    static const char *arrays[] = {"x", "y", "z", "z_l", "z_u", "s", "row_activity"};
    for (int32_t w = 0; w < 7; w++) {
        size_t len = 0;
        const double *a = optsolver_result_array(r, w, &len);
        put(arrays[w], a ? len : 0, a);
    }
    double v;
    if (optsolver_result_objective(r, &v)) put1("objective", v);
    if (optsolver_result_bound(r, &v)) put1("bound", v);
    if (optsolver_result_simplex_iterations(r, &v)) put1("simplex_iterations", v);
    int64_t it;
    if (optsolver_result_barrier_iterations(r, &it)) put1("barrier_iterations", (double)it);
    optsolver_timings t = optsolver_result_timings(r);
    double tv[4] = {t.load, t.solve, t.extraction, t.verification};
    put("timings", 4, tv);
    if (optsolver_result_has_basis(r)) {
        size_t n = (size_t)scalar("n", 0), m = (size_t)scalar("m", 0);
        double *bv = (double *)malloc((n + m + 1) * sizeof(double));
        for (size_t j = 0; j < n; j++) bv[j] = optsolver_result_basis_variable(r, j);
        put("basis_variables", n, bv);
        for (size_t i = 0; i < m; i++) bv[i] = optsolver_result_basis_row(r, i);
        put("basis_rows", m, bv);
        free(bv);
    }
    put1("certificate", optsolver_result_certificate(r));
    static const char *certs[] = {"cert_y", "cert_z", "cert_s", "cert_ray"};
    for (int32_t w = 0; w < 4; w++) {
        size_t len = 0;
        const double *a = optsolver_result_certificate_array(r, w, &len);
        put(certs[w], a ? len : 0, a);
    }
    size_t nr = optsolver_result_raw_status_count(r);
    put1("raw_status_count", (double)nr);
    for (size_t i = 0; i < nr; i++) {
        int64_t code = 0;
        const char *name = optsolver_result_raw_status(r, i, &code);
        char key[64];
        snprintf(key, sizeof key, "raw_status_%zu", i);
        puts_(key, name);
        snprintf(key, sizeof key, "raw_code_%zu", i);
        put1(key, (double)code);
    }
    puts_("solver_name", optsolver_result_solver_name(r));
    puts_("solver_version", optsolver_result_solver_version(r));
    size_t np = optsolver_result_parameter_count(r);
    put1("parameter_count", (double)np);
    for (size_t i = 0; i < np; i++) {
        optsolver_value val;
        const char *name = optsolver_result_parameter(r, i, &val);
        char key[64];
        snprintf(key, sizeof key, "parameter_%zu", i);
        char text[512];
        if (val.type == OPTSOLVER_PARAM_INT)
            snprintf(text, sizeof text, "%s=i%lld", name, (long long)val.int_value);
        else if (val.type == OPTSOLVER_PARAM_DOUBLE)
            snprintf(text, sizeof text, "%s=d%.17g", name, val.double_value);
        else
            snprintf(text, sizeof text, "%s=s%s", name, val.string_value ? val.string_value : "");
        puts_(key, text);
    }
    size_t na = optsolver_result_attribute_count(r);
    put1("attribute_count", (double)na);
    put1("verification", optsolver_result_verification(r));
    puts_("verification_name", optsolver_verification_name(optsolver_result_verification(r)));
    size_t nm = optsolver_result_measure_count(r);
    put1("measure_count", (double)nm);
    for (size_t i = 0; i < nm; i++) {
        optsolver_measure ms;
        if (optsolver_result_measure(r, i, &ms) != OPTSOLVER_OK) die("measure");
        char key[64];
        snprintf(key, sizeof key, "measure_name_%zu", i);
        puts_(key, ms.name);
        snprintf(key, sizeof key, "measure_%zu", i);
        double mv[5] = {ms.absolute, ms.relative, ms.tolerance, ms.verdict, ms.required};
        put(key, 5, mv);
    }
}

static optsolver_problem problem;
static optsolver_options options;
static optsolver_csc qm, fm;
static optsolver_cone *cones;
static optsolver_param *raw;

static void build(void) {
    memset(&problem, 0, sizeof problem);
    problem.struct_size = sizeof problem;
    size_t m = (size_t)scalar("m", 0), n = (size_t)scalar("n", 0);
    problem.A.rows = m;
    problem.A.cols = n;
    problem.A.col_ptrs = sizes("A_col_ptrs");
    if (scalar("index_i32", 0) != 0)
        problem.A_row_indices_i32 = ints32("A_row_indices");
    else
        problem.A.row_indices = sizes("A_row_indices");
    problem.A.values = dbl("A_values");
    problem.c = dbl("c");
    problem.sense = (int32_t)scalar("sense", 1);
    problem.gamma = scalar("gamma", 0);
    problem.l = dbl("l");
    problem.u = dbl("u");
    if (get("row_sense")) {
        item *it = get("row_sense");
        char *s = (char *)track(malloc(it->n + 1));
        for (size_t i = 0; i < it->n; i++) s[i] = (char)it->v[i];
        problem.row_sense = s;
        problem.b = dbl("b");
    }
    /* Both forms are passed when both are in the file (the library must reject that). */
    problem.b_l = dbl("b_l");
    problem.b_u = dbl("b_u");
    if (get("Q_col_ptrs")) {
        qm.rows = n;
        qm.cols = n;
        qm.col_ptrs = sizes("Q_col_ptrs");
        qm.row_indices = sizes("Q_row_indices");
        qm.values = dbl("Q_values");
        problem.Q = &qm;
    }
    if (get("F_col_ptrs")) {
        fm.rows = (size_t)scalar("k", 0);
        fm.cols = n;
        fm.col_ptrs = sizes("F_col_ptrs");
        fm.row_indices = sizes("F_row_indices");
        fm.values = dbl("F_values");
        problem.F = &fm;
        problem.g = dbl("g");
        item *kinds = get("cone_kinds"), *dims = get("cone_dims");
        size_t p = kinds ? kinds->n : 0;
        cones = (optsolver_cone *)track(calloc(p ? p : 1, sizeof *cones));
        for (size_t i = 0; i < p; i++) {
            cones[i].kind = (int32_t)kinds->v[i];
            cones[i].dim = (size_t)dims->v[i];
            char key[64];
            snprintf(key, sizeof key, "cone_exponents_%zu", i);
            item *e = get(key);
            if (e) {
                cones[i].exponents = e->v;
                cones[i].exponents_len = e->n;
            }
        }
        problem.cones = cones;
        problem.num_cones = p;
    }
    optsolver_options_init(&options);
    options.time_limit = scalar("opt_time_limit", NAN);
    options.iteration_limit = scalar("opt_iteration_limit", NAN);
    options.threads = (int64_t)scalar("opt_threads", -1);
    options.primal_feasibility_tolerance = scalar("opt_primal_feasibility_tolerance", NAN);
    options.dual_feasibility_tolerance = scalar("opt_dual_feasibility_tolerance", NAN);
    options.method = (int32_t)scalar("opt_method", -1);
    options.presolve = (int32_t)scalar("opt_presolve", -1);
    if (get("opt_random_seed")) {
        options.random_seed = (int64_t)scalar("opt_random_seed", 0);
        options.random_seed_set = 1;
    }
    options.solver_log = (int32_t)scalar("opt_solver_log", -1);
    options.array_outputs = (int32_t)scalar("opt_array_outputs", -1);
    options.verification = (int32_t)scalar("opt_verification", -1);
    options.certificates = (int32_t)scalar("opt_certificates", 0);
    options.memory_limit_gb = scalar("opt_memory_limit_gb", NAN);
    size_t nraw = (size_t)scalar("raw_count", 0);
    raw = (optsolver_param *)track(calloc(nraw ? nraw : 1, sizeof *raw));
    for (size_t i = 0; i < nraw; i++) {
        char key[64];
        snprintf(key, sizeof key, "str_raw_name_%zu", i);
        raw[i].name = get(key)->text;
        snprintf(key, sizeof key, "raw_value_%zu", i);
        item *v = get(key); /* type, number */
        raw[i].type = (int32_t)v->v[0];
        raw[i].int_value = (int64_t)v->v[1];
        raw[i].double_value = v->v[1];
        snprintf(key, sizeof key, "str_raw_string_%zu", i);
        item *s = get(key);
        raw[i].string_value = s ? s->text : NULL;
    }
    options.raw = raw;
    options.num_raw = nraw;
}

/* Buffer path: arrays of the given lengths (key "buf_<name>" = length, -1 or absent = none). */
static double *bufs[7];
static optsolver_buffers buffers;

static optsolver_array arr(int i, const char *key) {
    optsolver_array a = {NULL, 0};
    double len = scalar(key, -1);
    if (len >= 0) {
        a.len = (size_t)len;
        bufs[i] = (double *)track(malloc((a.len ? a.len : 1) * sizeof(double)));
        for (size_t j = 0; j < a.len; j++) bufs[i][j] = 12345.0; /* not NaN: the library must fill */
        a.data = bufs[i];
    }
    return a;
}

typedef struct job {
    optsolver_env *env;
    int32_t rc;
    optsolver_result *result;
    int free_env;
} job;

static int use_buffers;

static int32_t solve(optsolver_env *env, optsolver_result **res) {
    if (use_buffers) return optsolver_solve_into(env, &problem, &options, &buffers, res);
    return optsolver_solve(env, &problem, &options, res);
}

static void *thread_main(void *p) {
    job *j = (job *)p;
    j->rc = solve(j->env, &j->result);
    if (j->free_env) {
        optsolver_env_free(j->env);
        j->env = NULL;
    }
    return NULL;
}

/* Several problems through one handle: driver in1 out1 in2 out2 ... (the backend and options
 * of each file; the handle is created from the first file's backend). */
static int sequence(int argc, char **argv) {
    read_file(argv[1]);
    int32_t backend = (int32_t)scalar("backend", 0);
    uint64_t created0 = optsolver_environments_created();
    optsolver_env *env = NULL;
    int32_t rc = optsolver_env_create(backend, &env);
    for (int a = 1; a + 1 < argc; a += 2) {
        free_all();
        nitems = 0;
        read_file(argv[a]);
        out = fopen(argv[a + 1], "w");
        if (!out) die("cannot open result file");
        optsolver_result *res = NULL;
        int32_t r = rc;
        if (rc == OPTSOLVER_OK) {
            build();
            r = optsolver_solve(env, &problem, &options, &res);
        }
        write_error(r);
        put1("environments_created", (double)(optsolver_environments_created() - created0));
        if (r == OPTSOLVER_OK) write_result(res);
        optsolver_result_free(res);
        fclose(out);
    }
    optsolver_env_free(env);
    free_all();
    return 0;
}

int main(int argc, char **argv) {
    if (argc > 3 && argc % 2 == 1) return sequence(argc, argv);
    if (argc != 3) die("usage");
    read_file(argv[1]);
    out = fopen(argv[2], "w");
    if (!out) die("cannot open result file");
    put1("abi_version", optsolver_abi_version());
    int32_t backend = (int32_t)scalar("backend", 0);
    uint64_t created0 = optsolver_environments_created();

    if (scalar("caps", 0) != 0) {
        optsolver_caps *c = NULL;
        int32_t rc = optsolver_capabilities(backend, &c);
        write_error(rc);
        if (rc == OPTSOLVER_OK) {
            put1("caps_usable", optsolver_caps_usable(c));
            puts_("caps_reason", optsolver_caps_reason(c));
            put1("caps_classes", optsolver_caps_problem_classes(c));
            put1("caps_cones", optsolver_caps_cones(c));
            put1("caps_certificates", optsolver_caps_certificates(c));
            put1("caps_basis", optsolver_caps_basis(c));
            size_t no = optsolver_caps_option_count(c);
            put1("caps_option_count", (double)no);
            for (size_t i = 0; i < no; i++) {
                char key[64];
                snprintf(key, sizeof key, "caps_option_%zu", i);
                puts_(key, optsolver_caps_option(c, i));
            }
            puts_("caps_solver_name", optsolver_caps_solver_name(c));
            puts_("caps_solver_version", optsolver_caps_solver_version(c));
            optsolver_caps_free(c);
        }
        optsolver_env *e = NULL;
        int32_t rc2 = optsolver_env_create(backend, &e);
        put1("env_create_rc", rc2);
        optsolver_env_free(e);
        fclose(out);
        free_all();
        return 0;
    }

    build();
    use_buffers = scalar("buffers", 0) != 0;
    if (use_buffers) {
        buffers.struct_size = sizeof buffers;
        buffers.x = arr(0, "buf_x");
        buffers.y = arr(1, "buf_y");
        buffers.z = arr(2, "buf_z");
        buffers.row_activity = arr(3, "buf_row_activity");
        buffers.s = arr(4, "buf_s");
        buffers.z_l = arr(5, "buf_z_l");
        buffers.z_u = arr(6, "buf_z_u");
    }
    int once = scalar("once", 0) != 0;
    int repeats = (int)scalar("repeats", 1);
    int threaded = scalar("thread", 0) != 0;
    int32_t rc;
    optsolver_result *res = NULL;
    if (once) {
        rc = optsolver_solve_once(backend, &problem, &options, &res);
    } else {
        optsolver_env *env = NULL;
        rc = optsolver_env_create(backend, &env);
        if (rc == OPTSOLVER_OK) {
            if (threaded) {
                job j = {env, 0, NULL, 1};
                pthread_t t;
                if (pthread_create(&t, NULL, thread_main, &j) != 0) die("pthread_create");
                pthread_join(t, NULL);
                rc = j.rc;
                res = j.result;
                env = NULL;
            } else {
                for (int r = 0; r < repeats; r++) {
                    optsolver_result_free(res);
                    res = NULL;
                    rc = solve(env, &res);
                    if (rc != OPTSOLVER_OK) break;
                }
            }
        }
        optsolver_env_free(env);
    }
    write_error(rc);
    put1("environments_created", (double)(optsolver_environments_created() - created0));
    if (rc == OPTSOLVER_OK) write_result(res);
    if (use_buffers) {
        static const char *names[] = {"buf_x", "buf_y", "buf_z", "buf_row_activity", "buf_s", "buf_z_l", "buf_z_u"};
        static optsolver_array *as[7];
        as[0] = &buffers.x; as[1] = &buffers.y; as[2] = &buffers.z; as[3] = &buffers.row_activity;
        as[4] = &buffers.s; as[5] = &buffers.z_l; as[6] = &buffers.z_u;
        for (int i = 0; i < 7; i++) {
            char key[64];
            snprintf(key, sizeof key, "out_%s", names[i]);
            if (as[i]->data) put(key, as[i]->len, as[i]->data);
        }
    }
    if (scalar("leak_result", 0) == 0) optsolver_result_free(res);
    fclose(out);
    free_all();
    return 0;
}
