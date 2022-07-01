//! This pass transforms derefs of Box into a deref of the pointer inside Box.
//!
//! Box is not actually a pointer so it is incorrect to dereference it directly.

use crate::MirPass;
use rustc_index::vec::Idx;
use rustc_middle::mir::patch::MirPatch;
use rustc_middle::mir::visit::MutVisitor;
use rustc_middle::mir::*;
use rustc_middle::ty::{self, TyCtxt};

struct ElaborateBoxDerefVisitor<'tcx, 'a> {
    tcx: TyCtxt<'tcx>,
    local_decls: &'a mut LocalDecls<'tcx>,
    patch: MirPatch<'tcx>,
}

impl<'tcx, 'a> MutVisitor<'tcx> for ElaborateBoxDerefVisitor<'tcx, 'a> {
    fn tcx(&self) -> TyCtxt<'tcx> {
        self.tcx
    }

    fn visit_place(
        &mut self,
        place: &mut Place<'tcx>,
        context: visit::PlaceContext,
        location: Location,
    ) {
        let tcx = self.tcx;

        let base_ty = self.local_decls[place.local].ty;

        // Derefer ensures that derefs are always the first projection
        if place.projection.first() == Some(&PlaceElem::Deref) && base_ty.is_box() {
            let source_info = self.local_decls[place.local].source_info;

            let ptr_ty = tcx.mk_ty(ty::SuperPtr(base_ty.boxed_ty()));

            let ptr_local = self.patch.new_temp(ptr_ty, source_info.span);
            self.local_decls.push(LocalDecl::new(ptr_ty, source_info.span));

            self.patch.add_statement(location, StatementKind::StorageLive(ptr_local));

            self.patch.add_assign(
                location,
                Place::from(ptr_local),
                Rvalue::Use(Operand::Copy(tcx.mk_place_field(
                    Place::from(place.local),
                    Field::new(0),
                    ptr_ty,
                ))),
            );

            place.local = ptr_local;

            self.patch.add_statement(
                Location { block: location.block, statement_index: location.statement_index + 1 },
                StatementKind::StorageDead(ptr_local),
            );
        }

        self.super_place(place, context, location);
    }
}

pub struct ElaborateBoxDerefs;

impl<'tcx> MirPass<'tcx> for ElaborateBoxDerefs {
    fn run_pass(&self, tcx: TyCtxt<'tcx>, body: &mut Body<'tcx>) {
        if tcx.lang_items().owned_box().is_some() {
            let patch = MirPatch::new(body);

            let (basic_blocks, local_decls) = body.basic_blocks_and_local_decls_mut();

            let mut visitor = ElaborateBoxDerefVisitor { tcx, local_decls, patch };

            for (block, BasicBlockData { statements, terminator, .. }) in
                basic_blocks.iter_enumerated_mut()
            {
                let mut index = 0;
                for statement in statements {
                    let location = Location { block, statement_index: index };
                    visitor.visit_statement(statement, location);
                    index += 1;
                }

                if let Some(terminator) = terminator
                && !matches!(terminator.kind, TerminatorKind::Yield{..})
                {
                    let location = Location { block, statement_index: index };
                    visitor.visit_terminator(terminator, location);
                }

                let location = Location { block, statement_index: index };
                match terminator {
                    // yielding into a box is handled when lowering generators
                    Some(Terminator { kind: TerminatorKind::Yield { value, .. }, .. }) => {
                        visitor.visit_operand(value, location);
                    }
                    Some(terminator) => {
                        visitor.visit_terminator(terminator, location);
                    }
                    None => {}
                }
            }

            visitor.patch.apply(body);

            for debug_info in body.var_debug_info.iter_mut() {
                if let VarDebugInfoContents::Place(place) = &mut debug_info.value {
                    let mut new_projections: Option<Vec<_>> = None;
                    let mut last_deref = 0;

                    for (i, (base, elem)) in place.iter_projections().enumerate() {
                        let base_ty = base.ty(&body.local_decls, tcx).ty;

                        if elem == PlaceElem::Deref && base_ty.is_box() {
                            let new_projections = new_projections.get_or_insert_default();

                            let ptr_ty = tcx.mk_ty(ty::SuperPtr(base_ty.boxed_ty()));

                            new_projections.extend_from_slice(&base.projection[last_deref..]);
                            new_projections.push(PlaceElem::Field(Field::new(0), ptr_ty));
                            new_projections.push(PlaceElem::Deref);

                            last_deref = i;
                        }
                    }

                    if let Some(mut new_projections) = new_projections {
                        new_projections.extend_from_slice(&place.projection[last_deref..]);
                        place.projection = tcx.intern_place_elems(&new_projections);
                    }
                }
            }
        } else {
            // box is not present, this pass doesn't need to do anything
        }
    }
}
