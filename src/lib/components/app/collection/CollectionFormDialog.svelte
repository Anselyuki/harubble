<script lang="ts">
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';

  interface Props {
    open: boolean;
    mode: 'create' | 'edit';
    initialName?: string;
    initialDescription?: string;
    onSubmit: (name: string, description: string) => void | Promise<void>;
    onClose: () => void;
  }

  let {
    open = $bindable(),
    mode,
    initialName = '',
    initialDescription = '',
    onSubmit,
    onClose,
  }: Props = $props();

  let name = $state('');
  let description = $state('');
  let submitting = $state(false);

  $effect(() => {
    if (open) {
      name = initialName;
      description = initialDescription;
      submitting = false;
    }
  });

  const isValid = $derived.by(() => name.trim().length > 0);
  const title = $derived.by(() => {
    void localeState.current;
    return mode === 'create'
      ? m.collection_form_title_create()
      : m.collection_form_title_edit();
  });
  const submitLabel = $derived.by(() => {
    void localeState.current;
    return mode === 'create'
      ? m.collection_form_submit_create()
      : m.collection_form_submit_save();
  });

  async function handleSubmit() {
    if (!isValid || submitting) return;
    submitting = true;
    try {
      await onSubmit(name.trim(), description.trim());
      onClose();
    } catch {
      // Error handled by caller via notifyError
    } finally {
      submitting = false;
    }
  }
</script>

<Dialog.Root
  bind:open
  onOpenChange={(v) => {
    if (!v) onClose();
  }}
>
  <Dialog.Content class="collection-form-dialog">
    <Dialog.Header>
      <Dialog.Title>{title}</Dialog.Title>
    </Dialog.Header>
    <div class="dialog-body">
      <label class="settings-field">
        <span>{m.collection_form_label_name()}</span>
        <Input
          bind:value={name}
          placeholder={m.collection_form_placeholder_name()}
          class="h-9 border-[var(--dialog-border)] bg-[var(--dialog-control-bg)]"
        />
      </label>
      <label class="settings-field">
        <span>{m.collection_form_label_description()}</span>
        <Input
          bind:value={description}
          placeholder={m.collection_form_placeholder_description()}
          class="h-9 border-[var(--dialog-border)] bg-[var(--dialog-control-bg)]"
        />
      </label>
    </div>
    <Dialog.Footer>
      <Button variant="outline" onclick={onClose} disabled={submitting}>
        {m.collection_form_cancel()}
      </Button>
      <Button onclick={handleSubmit} disabled={!isValid || submitting}>
        {submitLabel}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
