#include "iree/hal/local/local_executable.h"

#include "iree/hal/local/executable_environment.h"

extern iree_status_t iree_hal_local_executable_issue_call(
    iree_hal_local_executable_t* executable, iree_host_size_t ordinal,
    const iree_hal_executable_dispatch_state_v0_t* dispatch_state,
    const iree_hal_executable_workgroup_state_v0_t* workgroup_state,
    uint32_t worker_id);

typedef void (*job_fn_ptr)(void*);

typedef enum {
    MUTEX_LOCKED=1,
    MUTEX_UNLOCKED,
    MUTEX_LOCKED_INIT=MUTEX_LOCKED,
} mutex_t;

typedef struct op_arg {
    iree_hal_local_executable_t* executable;
    iree_host_size_t ordinal;
    const iree_hal_executable_dispatch_state_v0_t* dispatch_state;
    iree_hal_executable_workgroup_state_v0_t workgroup_state;
    uint32_t worker_id;
} op_arg_t;

typedef struct job_state {
    op_arg_t op_arg;
    mutex_t done;
    iree_status_t status;
} job_state_t;

extern void defer_job(job_fn_ptr job_fn, void* job_arg);
// extern void defer_job(size_t job_fn, size_t job_arg);
extern void wait_job_done();
extern void set_job_num(size_t num);
extern void print_current_workgroup(size_t x, size_t y, size_t z);

//void defer_job(job_fn_ptr job_fn, void* job_arg) {
//    job_fn(job_arg);
//}

void mutex_lock(mutex_t* mutex) {
    //dummy func;
}

void mutex_unlock(mutex_t* mutex) {
    //dummy func;

}


void exec_dispatch_job(void* arg) {
    job_state_t* job_state = arg;

    job_state->status = iree_hal_local_executable_issue_call(
        job_state->op_arg.executable,
        job_state->op_arg.ordinal,
        job_state->op_arg.dispatch_state,
        &(job_state->op_arg.workgroup_state),
        job_state->op_arg.worker_id
    );

    mutex_unlock(&job_state->done);

}

#define N_JOBS 64
job_state_t job_states[N_JOBS] = {0};

extern void begin_record_op_latency();
extern void end_record_op_latency();

iree_status_t iree_hal_local_executable_issue_dispatch_inline(
    iree_hal_local_executable_t* executable, iree_host_size_t ordinal,
    const iree_hal_executable_dispatch_state_v0_t* dispatch_state,
    uint32_t processor_id, iree_byte_span_t local_memory) {

  const uint32_t workgroup_count_x = dispatch_state->workgroup_count_x;
  const uint32_t workgroup_count_y = dispatch_state->workgroup_count_y;
  const uint32_t workgroup_count_z = dispatch_state->workgroup_count_z;

  iree_status_t status = iree_ok_status();

//   iree_alignas(64) iree_hal_executable_workgroup_state_v0_t workgroup_state = {
//       .workgroup_id_x = 0,
//       .workgroup_id_y = 0,
//       .workgroup_id_z = 0,
//       .processor_id = processor_id,
//       .local_memory = local_memory.data,
//       .local_memory_size = (size_t)local_memory.data_length,
//   };

  int i = 0;
  begin_record_op_latency();
  set_job_num((workgroup_count_z)*(workgroup_count_y)*(workgroup_count_x));
  for (uint32_t z = 0; z < workgroup_count_z; ++z) {
    // workgroup_state.workgroup_id_z = z;
    for (uint32_t y = 0; y < workgroup_count_y; ++y) {
    //   workgroup_state.workgroup_id_y = y;
      for (uint32_t x = 0; x < workgroup_count_x; ++x) {
        // workgroup_state.workgroup_id_x = x;

        // status = iree_hal_local_executable_issue_call(
        //     executable, ordinal, dispatch_state, &workgroup_state,
        //     /*worker_id=*/0);
        // if (!iree_status_is_ok(status)) break;
        job_states[i].done = MUTEX_LOCKED_INIT;
        job_states[i].op_arg.executable = executable;
        job_states[i].op_arg.ordinal = ordinal;
        job_states[i].op_arg.dispatch_state = dispatch_state;
        
        job_states[i].op_arg.workgroup_state.workgroup_id_x = x,
        job_states[i].op_arg.workgroup_state.workgroup_id_y = y,
        job_states[i].op_arg.workgroup_state.workgroup_id_z = z,
        job_states[i].op_arg.workgroup_state.processor_id = processor_id,
        job_states[i].op_arg.workgroup_state.local_memory = local_memory.data,
        job_states[i].op_arg.workgroup_state.local_memory_size = (size_t)local_memory.data_length,
        job_states[i].op_arg.worker_id = 0;

        job_states[i].status = iree_ok_status();
        print_current_workgroup(x,y,z);
        defer_job(exec_dispatch_job, &job_states[i]);

        i++;

      }
    }
  }
  wait_job_done();
  end_record_op_latency();
  for (int j = 0; j < i; j++) {

    status = job_states[j].status;
    if (!iree_status_is_ok(status)) break;
  }
  
  return status;
}
