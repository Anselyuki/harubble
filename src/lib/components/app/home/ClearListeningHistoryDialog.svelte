<script lang="ts">
  import * as AlertDialog from '$lib/components/ui/alert-dialog/index.js';
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';

  interface Props {
    open: boolean;
    onOpenChange: (open: boolean) => void;
    onConfirm: () => void | Promise<void>;
  }

  let { open, onOpenChange, onConfirm }: Props = $props();
  let isClearing = $state(false);

  const labels = $derived.by(() => {
    void localeState.current;
    return {
      title: m.home_clear_history_dialog_title(),
      description: m.home_clear_history_dialog_description(),
      cancel: m.home_clear_history_dialog_cancel(),
      confirm: m.home_clear_history_dialog_confirm(),
    };
  });

  async function handleConfirm() {
    if (isClearing) return;
    isClearing = true;
    try {
      await onConfirm();
    } finally {
      isClearing = false;
    }
  }
</script>

<AlertDialog.Root {open} {onOpenChange}>
  <AlertDialog.Content>
    <AlertDialog.Header>
      <AlertDialog.Title>{labels.title}</AlertDialog.Title>
      <AlertDialog.Description>{labels.description}</AlertDialog.Description>
    </AlertDialog.Header>
    <AlertDialog.Footer>
      <AlertDialog.Cancel disabled={isClearing}
        >{labels.cancel}</AlertDialog.Cancel
      >
      <AlertDialog.Action
        variant="destructive"
        disabled={isClearing}
        onclick={() => void handleConfirm()}
        >{labels.confirm}</AlertDialog.Action
      >
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>
